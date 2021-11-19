#if defined _voiceserver_included
	#endinput
#endif
#define _voiceserver_included

native void ClientToVoiceVolumeMap(float volume[64], bool set);

public Extension __ext_voiceserver = 
{
	name = "VoiceServer",
	file = "voiceserver.ext",
#if defined AUTOLOAD_EXTENSIONS
	autoload = 1,
#else
	autoload = 0,
#endif
#if defined REQUIRE_EXTENSIONS
	required = 1,
#else
	required = 0,
#endif
};

#if !defined REQUIRE_EXTENSIONS
public void __ext_voiceserver_SetNTVOptional()
{
	MarkNativeAsOptional("ClientToVoiceVolumeMap");
}
#endif
