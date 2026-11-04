#pragma once

// Frame layout on UART: SOF | type | len | payload[len] | checksum
// checksum = XOR of type, len and payload bytes.
// Payload is a raw HID report (ble_hid_keyboard_report_t / ble_hid_mouse_report_t).

#define SERIAL_HID_SOF 0xAA

#define SERIAL_HID_KEYBOARD_LEN 8
#define SERIAL_HID_MOUSE_LEN    4

typedef enum {
    SERIAL_HID_TYPE_KEYBOARD = 0x01,
    SERIAL_HID_TYPE_MOUSE = 0x02,
} serial_hid_type_t;
