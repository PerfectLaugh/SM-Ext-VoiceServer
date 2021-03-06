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

#define VOICESERVER_FAKECLIENT_NAME "Sympho"

void* engineFactory = nullptr;

ISDKTools *sdktools = nullptr;
IServer *iserver = nullptr;

float *g_fClientVolumeMap = nullptr;

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

DETOUR_DECL_STATIC3(SV_BroadcastVoiceData, void, IClient*, cl, CCLCMsg_VoiceData&, msg, bool, unk)
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
    auto volume = 1.0;
    if (g_fClientVolumeMap) {
    	volume = g_fClientVolumeMap[client_index];
    }

	rust::Slice<const uint8_t> slice((const uint8_t*)msg.data().c_str(), msg.data().size());
	auto data = ext::on_recv_voicedata(client_index, volume, steamid, slice);

	msg.mutable_data()->assign((const char*)data.data(), data.size());

	DETOUR_STATIC_CALL(SV_BroadcastVoiceData)(cl, msg, unk);
	return;
}

static void OnGameFrame(bool simulating) {
	ext::on_gameframe();
}

extern const sp_nativeinfo_t g_Natives[];

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
	    	addr_cfg = "";
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

		ext::init(addr_cfg);

		CDetourManager::Init(smutils->GetScriptingEngine(), nullptr);
		g_SV_BroadcastVoiceData_Detour = DETOUR_CREATE_STATIC(SV_BroadcastVoiceData, pattern);

		smutils->AddGameFrameHook(&OnGameFrame);

		sharesys->AddNatives(myself, g_Natives);
		sharesys->RegisterLibrary(myself, "VoiceServer");

		g_SV_BroadcastVoiceData_Detour->EnableDetour();

		return true;
	}

	virtual void SDK_OnUnload() {
		if (g_SV_BroadcastVoiceData_Detour) {
			g_SV_BroadcastVoiceData_Detour->Destroy();
			g_SV_BroadcastVoiceData_Detour = nullptr;
		}

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

static cell_t Native_ClientToVoiceVolumeMap(IPluginContext *pContext, const cell_t *params)
{
	if(params[2])
		pContext->LocalToPhysAddr(params[1], (cell_t **)&g_fClientVolumeMap);
	else
		g_fClientVolumeMap = nullptr;

	return 0;
}

const sp_nativeinfo_t g_Natives[] = 
{
	{ "ClientToVoiceVolumeMap", Native_ClientToVoiceVolumeMap },
	{ nullptr, nullptr },
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
        if (audio_data.size() <= 0) {
        	return;
        }

        CCLCMsg_VoiceData msg;
        msg.set_data((const char*)audio_data.data(), audio_data.size());

        DETOUR_STATIC_CALL(SV_BroadcastVoiceData)(cl, msg, false);
	}

	void log_error(rust::Str msg) {
		std::string msg_str(msg.data(), msg.size());
		smutils->LogError(myself, "%s", msg_str.c_str());
	}
}

Ext g_Ext;
SMEXT_LINK(&g_Ext);
