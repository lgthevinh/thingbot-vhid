#pragma once

#include <stdbool.h>
#include <stdint.h>
#include <stddef.h>
#include "esp_err.h"

#define BLE_HID_REPORT_ID_KEYBOARD 1
#define BLE_HID_REPORT_ID_MOUSE    2

typedef struct {
    uint8_t modifiers;
    uint8_t reserved;
    uint8_t keys[6];
} ble_hid_keyboard_report_t;

typedef struct {
    uint8_t leds;
} ble_hid_keyboard_leds_t;

typedef struct {
    uint8_t buttons;
    int8_t x;
    int8_t y;
    int8_t wheel;
} ble_hid_mouse_report_t;

esp_err_t ble_hid_init(const char *device_name);
bool ble_hid_is_connected(void);
esp_err_t ble_hid_send_keyboard(const ble_hid_keyboard_report_t *report);
esp_err_t ble_hid_send_mouse(const ble_hid_mouse_report_t *report);
