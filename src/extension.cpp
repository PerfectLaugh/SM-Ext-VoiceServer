#include "extension.h"
#include "extensions/ISDKTools.h"

#include <memory>
#include <string>

#include "voiceserver-ext/src/extension.rs.h"

#include <iserver.h>
#include <iclient.h>
#include <inetmessage.h>
#include <protobuf/netmessages.pb.h>

#include <CDetour/detours.h>

#include "ivoicecodec.h"

#ifdef _WIN32
	#define VAUDIO_LIBRARY "bin/vaudio_celt.dll"
#else
    #define VAUDIO_LIBRARY "bin/vaudio_celt_client.so"
#endif
#define VAUDIO_QUALITY (3)
#define VOICESERVER_FAKECLIENT_NAME "Sympho"

void* engineFactory = nullptr;

ISDKTools *sdktools = nullptr;
IServer *iserver = nullptr;

CDetour *g_SV_BroadcastVoiceData_Detour = nullptr;

#define MAXPLAYERS (64)

static inline void *GetCGameClientFromIClient(IClient *iclient)
{
	// The IClient vtable is +sizeof(void *) from the CBaseClient vtable due to multiple inheritance.
	return (void*)((intptr_t)iclient - sizeof(void*));
}

static inline IClient *GetIClientFromCGameClient(void *cgameclient)
{
	return (IClient*)((intptr_t)cgameclient + sizeof(void*));
}

#ifdef _WIN32
#include <Windows.h>
#else
#include <dlfcn.h>
#endif

#ifdef _WIN32
    typedef HMODULE LIBTYPE;
#else
    typedef void* LIBTYPE;
#endif

LIBTYPE g_pCodecLib = nullptr;
std::unique_ptr<IVoiceCodec> g_pCodecArray[MAXPLAYERS];

DETOUR_DECL_STATIC3(SV_BroadcastVoiceData, void, IClient*, cl, const CCLCMsg_VoiceData&, msg, bool, unk)
{
	auto client_index = cl->GetPlayerSlot();
	if (client_index < 0 || client_index >= MAXPLAYERS) {
		DETOUR_STATIC_CALL(SV_BroadcastVoiceData)(cl, msg, unk);
		return;
	}

	auto player = playerhelpers->GetGamePlayer(client_index + 1);
    if (player == nullptr || !player->IsConnected() || !player->IsInGame()) {
    	DETOUR_STATIC_CALL(SV_BroadcastVoiceData)(cl, msg, unk);
		return;
    }

    auto steamid = player->GetSteamId64();

	static char decompressed[1024 * 100];
	int decompressed_size = g_pCodecArray[client_index]->Decompress(msg.data().c_str(), msg.data().size(), decompressed, sizeof(decompressed));

	rust::Slice<const uint8_t> slice((const uint8_t*)decompressed, decompressed_size * sizeof(int16_t));
	ext::on_recv_voicedata(steamid, slice);

	DETOUR_STATIC_CALL(SV_BroadcastVoiceData)(cl, msg, unk);
	return;
}

static void OnGameFrame(bool simulating) {
	ext::on_gameframe();
}

class Ext : public SDKExtension
{
public:
	virtual bool SDK_OnMetamodLoad(ISmmAPI *ismm, char *error, size_t maxlen, bool late) {
		engineFactory = reinterpret_cast<void*>(ismm->GetEngineFactory(false));
		if (engineFactory == nullptr) {
			smutils->Format(error, maxlen, "Could not load engineFactory from metamod");
			return false;
		}
		return true;
	}

	virtual bool SDK_OnLoad(char *error, size_t maxlength, bool late) {
		auto addr_cfg = smutils->GetCoreConfigValue("VoiceServerListenAddress");
	    if (addr_cfg == nullptr) {
	    	smutils->Format(error, maxlength, "No listen address specified in `VoiceServerListenAddress` in core.cfg");
	    	return false;
	    }

#ifdef _WIN32
		auto pattern = memutils->FindPattern(engineFactory, "", 0);
#else
		auto pattern = memutils->FindPattern(engineFactory, "\x55\x89\xE5\x57\x56\x8D\x55\x2A\x53\x81\xEC\xEC\x00\x00\x00", 15);
#endif
		if (pattern == nullptr) {
			smutils->Format(error, maxlength, "Could not find SV_BroadcastVoiceData from engine");
			return false;
		}

#ifdef _WIN32
	    g_pCodecLib = LoadLibrary(VAUDIO_LIBRARY);
	    auto createInterface = (CreateInterfaceFn)GetProcAddress(g_pCodecLib, "CreateInterface");
#else
	    g_pCodecLib = dlopen(VAUDIO_LIBRARY, RTLD_LAZY);
	    auto createInterface = (CreateInterfaceFn)dlsym(g_pCodecLib, "CreateInterface");
#endif

	    if (createInterface == nullptr) {
	    	smutils->Format(error, maxlength, "Could not initialize codec library");
	        return false;
	    }

	    for (int i = 0; i < MAXPLAYERS; i++) {
	    	auto rawcodec = reinterpret_cast<IVoiceCodec*>(createInterface("vaudio_celt", NULL));
		    if (rawcodec == nullptr) {
		    	smutils->Format(error, maxlength, "Could not initialize codec from codec library");
		    	return false;
		    }

		    g_pCodecArray[i] = std::move(std::unique_ptr<IVoiceCodec>(rawcodec));
		    g_pCodecArray[i]->Init(VAUDIO_QUALITY);
	    }

		ext::init(addr_cfg);

		CDetourManager::Init(smutils->GetScriptingEngine(), nullptr);
		g_SV_BroadcastVoiceData_Detour = DETOUR_CREATE_STATIC(SV_BroadcastVoiceData, pattern);

		smutils->AddGameFrameHook(&OnGameFrame);

		g_SV_BroadcastVoiceData_Detour->EnableDetour();

		return true;
	}

	virtual void SDK_OnUnload() {
		if (g_SV_BroadcastVoiceData_Detour) {
			g_SV_BroadcastVoiceData_Detour->Destroy();
			g_SV_BroadcastVoiceData_Detour = nullptr;
		}

		for (int i = 0; i < MAXPLAYERS; i++) {
		    g_pCodecArray[i].reset(nullptr);
	    }

#ifdef _WIN32
	    FreeLibrary(g_pCodecLib);
#else
	    dlclose(g_pCodecLib);
#endif

		smutils->RemoveGameFrameHook(&OnGameFrame);

		ext::shutdown();
	}

	void SDK_OnAllLoaded() {
        SM_GET_LATE_IFACE(SDKTOOLS, sdktools);
        if (sdktools == nullptr) {
            smutils->LogError(myself, "Cannot get sdktools instance.");
            return;
        }

        iserver = sdktools->GetIServer();
    }
};

namespace ext {
	void send_client_voice(int32_t client_index, rust::Slice<const uint8_t> audio_data) {
		if (iserver == nullptr) {
			return;
		}
		if (client_index < -1 || client_index >= MAXPLAYERS) {
			return;
		}

		if (client_index == -1) {
			static int bot_index = -1;
			if (bot_index != -1) {
				auto player = playerhelpers->GetGamePlayer(bot_index + 1);
				if (player == nullptr || !player->IsConnected() || !player->IsInGame() || !player->IsFakeClient()) {
					bot_index = -1;
				}
			}
			if (bot_index == -1) {
				auto edict = engine->CreateFakeClient(VOICESERVER_FAKECLIENT_NAME);
				if (edict == nullptr) {
					return;
				}
				auto player = playerhelpers->GetGamePlayer(edict);
				bot_index = player->GetIndex() - 1;
			}
			client_index = bot_index;
		}

        auto player = playerhelpers->GetGamePlayer(client_index + 1);
        if (player == nullptr || !player->IsConnected() || !player->IsInGame()) {
        	return;
        }
        IClient* cl = iserver->GetClient(client_index);
        if (cl == nullptr) {
            return;
        }

        int data_size = (int)audio_data.size();
        if (data_size <= 0) {
        	return;
        }

        auto compressed = new char[data_size];
        int compressed_size = g_pCodecArray[client_index]->Compress((const char*)audio_data.data(), data_size / sizeof(int16_t), compressed, data_size, false);

        CCLCMsg_VoiceData msg;
        msg.set_data(compressed, compressed_size);

        DETOUR_STATIC_CALL(SV_BroadcastVoiceData)(cl, msg, false);

        delete[] compressed;
	}

	void log_error(rust::Str msg) {
		std::string msg_str(msg.data(), msg.size());
		smutils->LogError(myself, "%s", msg_str.c_str());
	}
}

Ext g_Ext;
SMEXT_LINK(&g_Ext);
