#pragma once

#include "esp_err.h"

// Starts the UART0 listener task that forwards valid frames to BLE HID.
esp_err_t serial_listener_init(void);
