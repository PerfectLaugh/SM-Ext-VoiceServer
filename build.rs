use std::env::var;
use std::path::Path;

#[cfg(feature = "metamod")]
mod metamod {
    use std::collections::HashMap;
    use std::env::var;
    use std::path::Path;

    use lazy_static::lazy_static;

    pub struct Sdk {
        pub path: &'static str,
        pub code: &'static str,
        pub define: &'static str,
    }

    impl Sdk {
        fn new(path: &'static str, code: &'static str, define: &'static str) -> Sdk {
            Sdk { path, code, define }
        }
    }

    lazy_static! {
        static ref POSSIBLE_SDKS: HashMap<&'static str, Sdk> = {
            let mut map = HashMap::new();
            map.insert("episode1", Sdk::new("HL2SDK", "1", "EPISODEONE"));
            map.insert("ep2", Sdk::new("HL2SDKOB", "3", "ORANGEBOX"));
            map.insert("css", Sdk::new("HL2SDKCSS", "6", "CSS"));
            map.insert("hl2dm", Sdk::new("HL2SDKHL2DM", "7", "HL2DM"));
            map.insert("dods", Sdk::new("HL2SDKDODS", "8", "DODS"));
            map.insert("sdk2013", Sdk::new("HL2SDK2013", "9", "SDK2013"));
            map.insert("tf2", Sdk::new("HL2SDKTF2", "11", "TF2"));
            map.insert("l4d", Sdk::new("HL2SDKL4D", "12", "LEFT4DEAD"));
            map.insert("nucleardawn", Sdk::new("HL2SDKND", "13", "NUCLEARDAWN"));
            map.insert("l4d2", Sdk::new("HL2SDKL4D2", "15", "LEFT4DEAD2"));
            map.insert("darkm", Sdk::new("HL2SDK-DARKM", "2", "DARKMESSIAH"));
            map.insert("swarm", Sdk::new("HL2SDK-SWARM", "16", "ALIENSWARM"));
            map.insert("bgt", Sdk::new("HL2SDK-BGT", "4", "BLOODYGOODTIME"));
            map.insert("eye", Sdk::new("HL2SDK-EYE", "5", "EYE"));
            map.insert("csgo", Sdk::new("HL2SDKCSGO", "21", "CSGO"));
            map.insert("portal2", Sdk::new("HL2SDKPORTAL2", "17", "PORTAL2"));
            map.insert("blade", Sdk::new("HL2SDKBLADE", "18", "BLADE"));
            map.insert(
                "insurgency",
                Sdk::new("HL2SDKINSURGENCY", "19", "INSURGENCY"),
            );
            map.insert("contagion", Sdk::new("HL2SDKCONTAGION", "14", "CONTAGION"));
            map.insert("bms", Sdk::new("HL2SDKBMS", "10", "BMS"));
            map.insert("doi", Sdk::new("HL2SDKDOI", "20", "DOI"));
            map
        };
    }

    pub fn get_sdk() -> &'static Sdk {
        let mut sdk = None;
        if cfg!(feature = "csgo") {
            sdk = Some(POSSIBLE_SDKS.get("csgo").unwrap())
        }

        sdk.unwrap()
    }

    pub fn configure_for_hl2<P: AsRef<Path>>(mm_root: P, config: &mut cc::Build) {
        let mms_path = if cfg!(feature = "episode1") {
            mm_root.as_ref().join("core-legacy")
        } else {
            mm_root.as_ref().join("core")
        };

        config.include(&mms_path);
        config.include(mms_path.join("sourcehook"));

        let sdk = get_sdk();

        let sdk_path = var(sdk.path).unwrap();
        let sdk_path = Path::new(&sdk_path);
        config.define(format!("SE_{}", sdk.define).as_str(), Some(sdk.code));

        config.include(sdk_path.join("public"));
        config.include(sdk_path.join("public/engine"));
        config.include(sdk_path.join("public/mathlib"));
        config.include(sdk_path.join("public/vstdlib"));
        config.include(sdk_path.join("public/tier0"));
        config.include(sdk_path.join("public/tier1"));

        config.include(sdk_path.join("public/game/server"));
        config.include(sdk_path.join("public/toolframework"));
        config.include(sdk_path.join("game/shared"));
        config.include(sdk_path.join("common"));

        #[cfg(target_env = "msvc")]
        {
            config.define("COMPILER_MSVC", None);
            config.define("COMPILER_MSVC32", None);
        }

        #[cfg(target_env = "gnu")]
        {
            config.define("COMPILER_GCC", None);
        }
    }
}

fn configure_build<P: AsRef<Path>>(sm_root: P, config: &mut cc::Build) {
    let sm_root = sm_root.as_ref();

    #[cfg(not(debug_assertions))]
    {
        config.define("NDEBUG", None);
    }

    #[cfg(target_env = "gnu")]
    {
        config.define("stricmp", Some("strcasecmp"));
        config.define("_stricmp", Some("strcasecmp"));
        config.define("_snprintf", Some("snprintf"));
        config.define("_vsnprintf", Some("vsnprintf"));
        config.define("HAVE_STDINT_H", None);
        config.define("GNUC", None);

        config.flag("-pipe");
        config.flag("-fno-strict-aliasing");
        config.flag("-Wall");
        //config.flag("-Werror")
        config.flag("-Wno-unused");
        config.flag("-Wno-switch");
        config.flag("-Wno-array-bounds");
        config.flag("-msse");
        config.flag("-m32");
        config.flag("-fvisibility=hidden");

        config.flag("-std=c++14");
        config.flag("-fno-threadsafe-statics");
        config.flag("-Wno-non-virtual-dtor");
        config.flag("-Wno-overloaded-virtual");
        config.flag("-fvisibility-inlines-hidden");

        //config.flag_if_supported("-Wno-inconsistent-missing-override");
        config.flag_if_supported("-Wno-narrowing");
        config.flag_if_supported("-Wno-delete-non-virtual-dtor");
        config.flag_if_supported("-Wno-unused-result");
        config.flag_if_supported("-Wno-sized-deallocation");

        //config.flag_if_supported("-Wno-implicit-exception-spec-mismatch");
        //config.flag_if_supported("-Wno-deprecated-register");
        config.flag_if_supported("-Wno-deprecated");
        //config.flag_if_supported("-Wno-sometimes-uninitialized");

        config.flag_if_supported("-mfpmath=sse");
    }
    #[cfg(target_env = "msvc")]
    {
        config.define("_CRT_SECURE_NO_DEPRECATE", None);
        config.define("_CRT_SECURE_NO_WARNINGS", None);
        config.define("_CRT_NONSTDC_NO_DEPRECATE", None);
        config.define("_ITERATOR_DEBUG_LEVEL", Some("0"));

        config.flag("/EHsc");
        config.flag("/GR-");
        config.flag("/TP");

        println!("cargo:rustc-link-lib=legacy_stdio_definitions");

        config.force_frame_pointer(true);
    }

    #[cfg(target_os = "windows")]
    {
        config.define("WIN32", None);
        config.define("_WINDOWS", None);
    }
    #[cfg(target_os = "linux")]
    {
        config.define("_LINUX", None);
        config.define("POSIX", None);
        config.flag("-Wl,--exclude-libs,ALL");
        config.flag("-lm");
        config.flag_if_supported("-static-libgcc");
        config.flag_if_supported("-lgcc_eh");

        config.define("_GLIBCXX_USE_CXX11_ABI", Some("0"));
    }
    #[cfg(target_os = "mac")]
    {
        config.define("OSX", None);
        config.define("_OSX", None);
        config.define("POSIX", None);
        config.flag("-mmacosx-version-min=10.5");
        config.flag("-arch=i386");
        config.flag("-lstdc++");
        config.flag("-stdlib=libstdc++");
    }

    config
        .include("src")
        .include(sm_root.join("public"))
        .include(sm_root.join("public/extensions"))
        .include(sm_root.join("sourcepawn/include"))
        .include(sm_root.join("public/amtl/amtl"))
        .include(sm_root.join("public/amtl"));
}

#[cfg(feature = "protobuf")]
fn configure_protobuf<P: AsRef<Path>>(sdk_path: P, config: &mut cc::Build) {
    let sdk_path = sdk_path.as_ref();

    config.include(sdk_path.join("common/protobuf-2.5.0/src"));
    config.file(sdk_path.join("public/engine/protobuf/netmessages.pb.cc"));
}

fn main() {
    tonic_build::configure()
        .compile(&["proto/voiceserver/voiceserver.proto"], &["proto"])
        .unwrap();

    println!("cargo:rerun-if-changed=src/extension.h");
    println!("cargo:rerun-if-changed=src/extension.cpp");
    println!("cargo:rerun-if-changed=src/smsdk_config.h");

    let mut config = cxx_build::bridge("src/extension.rs");

    let sm_root_string = var("SOURCEMOD18")
        .or_else(|_| var("SOURCEMOD"))
        .or_else(|_| var("SOURCEMOD_DEV"))
        .unwrap();

    let sm_root = Path::new(&sm_root_string);

    configure_build(&sm_root, &mut config);

    let smsdk_ext_data = std::fs::read_to_string(sm_root.join("public/smsdk_ext.cpp")).unwrap();
    let smsdk_ext_data = smsdk_ext_data.replace("GetSMExtAPI", "GetSMExtAPI_Internal");
    let smsdk_ext_data = smsdk_ext_data.replace("PL_EXPOSURE", "CreateInterface_Internal");

    {
        use std::io::Write;
        let mut file =
            std::fs::File::create(format!("{}/smsdk_ext.cpp", var("OUT_DIR").unwrap())).unwrap();
        file.write_all(smsdk_ext_data.as_bytes()).unwrap();
    }

    config.file("src/extension.cpp");
    config.file(format!("{}/smsdk_ext.cpp", var("OUT_DIR").unwrap()));
    config.define(
        "EXTENSION_VERSION",
        format!("\"{}\"", env!("CARGO_PKG_VERSION")).as_ref(),
    );
    config.file(sm_root.join("public/CDetour/detours.cpp"));

    #[cfg(feature = "metamod")]
    {
        let mm_root = var("MMSOURCE110")
            .or_else(|_| var("MMSOURCE"))
            .or_else(|_| var("MMSOURCE_DEV"))
            .ok();

        metamod::configure_for_hl2(mm_root.as_ref().unwrap(), &mut config);

        #[cfg(feature = "protobuf")]
        configure_protobuf(var(metamod::get_sdk().path).unwrap(), &mut config);
    }

    config.compile("extension");

    let mut asm = cc::Build::new();
    #[cfg(target_os = "windows")]
    {
        asm.define("WIN32", None);
        asm.define("_WINDOWS", None);
    }

    asm.include(sm_root.join("public"));
    asm.file(sm_root.join("public/asm/asm.c"));
    asm.file(sm_root.join("public/libudis86/decode.c"));
    asm.file(sm_root.join("public/libudis86/itab.c"));
    asm.file(sm_root.join("public/libudis86/syn-att.c"));
    asm.file(sm_root.join("public/libudis86/syn-intel.c"));
    asm.file(sm_root.join("public/libudis86/syn.c"));
    asm.file(sm_root.join("public/libudis86/udis86.c"));

    asm.compile("asm");
}
