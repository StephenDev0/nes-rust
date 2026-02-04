#ifndef NesRustBridge_h
#define NesRustBridge_h

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// Basic NES API (for direct Nes pointer usage)
void* nes_create(void);
void nes_destroy(void* nes_ptr);
void nes_load_rom(void* nes_ptr, const uint8_t* data, size_t len);
void nes_reset(void* nes_ptr);
void nes_step_frame(void* nes_ptr);
void nes_get_pixels(void* nes_ptr, uint8_t* buffer, size_t len);
void nes_get_audio_samples(void* nes_ptr, float* buffer, size_t len);
void nes_input(void* nes_ptr, int button_id, int pressed);

// Save state API (for direct Nes pointer usage)
int nes_save_state(void* nes_ptr, const char* path);
int nes_load_state(void* nes_ptr, const char* path);
size_t nes_save_state_to_buffer(void* nes_ptr, uint8_t* buffer, size_t buffer_len);
int nes_load_state_from_buffer(void* nes_ptr, const uint8_t* buffer, size_t buffer_len);

// iOS threading model API (uses EmuState for thread-safe operation)
typedef void EmuState;
EmuState* initEmu(const char* rom_path);
void runEmuLoop(EmuState* state);
void renderFrame(EmuState* state);
void cleanupEmu(EmuState* state);
void stopEmu(EmuState* state);
void resetEmu(EmuState* state);
void setEmuPaused(EmuState* state, int paused);
int nes_is_ds(EmuState* state);

// Save state API (for iOS EmuState)
int saveEmuState(EmuState* state, const char* path);
int loadEmuState(EmuState* state, const char* path);

// Virtual button input (global, thread-safe)
void set_virtual_button_state(int button_id, int pressed);

// Touch input for DS (global, thread-safe)
void nes_touch(EmuState* state, int x, int y, int pressed);

// Legacy function
int startEmu(const char* rom_path, const char* state_path, int slot_count, int initial_slot);

#ifdef __cplusplus
}
#endif

#endif
