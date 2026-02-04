#ifndef NesRustBridge_h
#define NesRustBridge_h

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void* nes_create(void);
void nes_destroy(void* nes_ptr);
void nes_load_rom(void* nes_ptr, const uint8_t* data, size_t len);
void nes_reset(void* nes_ptr);
void nes_step_frame(void* nes_ptr);
void nes_get_pixels(void* nes_ptr, uint8_t* buffer, size_t len);
void nes_get_audio_samples(void* nes_ptr, float* buffer, size_t len);
void nes_input(void* nes_ptr, int button_id, int pressed);

#ifdef __cplusplus
}
#endif

#endif