#ifndef EXT_EXTENSION_H_
#define EXT_EXTENSION_H_

#include "smsdk_ext.h"

#include "rust/cxx.h"

namespace ext {

void send_client_voice(int32_t client_index, rust::Slice<const uint8_t> audio_data);

void log_error(rust::Str msg);

}

#endif // EXT_EXTENSION_H_
