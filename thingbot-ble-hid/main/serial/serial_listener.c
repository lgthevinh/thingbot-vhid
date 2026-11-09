#include "serial_listener.h"
#include "serial_listener_define.h"

#include <string.h>

#include "driver/usb_serial_jtag.h"
#include "esp_check.h"
#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"

#include "ble_hid.h"

// The board's USB port is the chip's USB-Serial-JTAG, not a bridge to UART0; logs share the same port
#define SERIAL_LISTENER_RX_BUF  1024
#define SERIAL_LISTENER_MAX_LEN SERIAL_HID_KEYBOARD_LEN

static const char *TAG = "serial_listener";

typedef enum {
    PARSE_SOF,
    PARSE_TYPE,
    PARSE_LEN,
    PARSE_PAYLOAD,
    PARSE_CHECKSUM,
} parse_state_t;

/**
 * Forwards a validated frame's payload as a BLE HID report.
 * Input is dropped while no host is connected so stale keys are never replayed later.
 */
static void serial_listener_dispatch(uint8_t type, const uint8_t *payload)
{
    if (!ble_hid_is_connected()) {
        return;
    }

    if (type == SERIAL_HID_TYPE_KEYBOARD) {
        ble_hid_keyboard_report_t report;
        memcpy(&report, payload, sizeof(report));
        ble_hid_send_keyboard(&report);
    } else {
        ble_hid_mouse_report_t report;
        memcpy(&report, payload, sizeof(report));
        ble_hid_send_mouse(&report);
    }
}

/**
 * Reads USB-Serial-JTAG bytes and runs them through the frame parser.
 * Any invalid type, length or checksum resets to SOF search, which is how the stream resyncs.
 */
static void serial_listener_task(void *param)
{
    parse_state_t state = PARSE_SOF;
    uint8_t type = 0, len = 0, pos = 0, sum = 0;
    uint8_t payload[SERIAL_LISTENER_MAX_LEN];
    uint8_t buf[64];

    for (;;) {
        int n = usb_serial_jtag_read_bytes(buf, sizeof(buf), pdMS_TO_TICKS(10));

        for (int i = 0; i < n; i++) {
            uint8_t b = buf[i];

            switch (state) {
            case PARSE_SOF:
                if (b == SERIAL_HID_SOF) {
                    state = PARSE_TYPE;
                }
                break;

            case PARSE_TYPE:
                type = b;
                sum = b;
                if (type == SERIAL_HID_TYPE_KEYBOARD || type == SERIAL_HID_TYPE_MOUSE) {
                    state = PARSE_LEN;
                } else {
                    state = PARSE_SOF;
                }
                break;

            case PARSE_LEN: {
                // length is fixed per type, so a mismatch means we synced on a stray 0xAA
                uint8_t expected;
                if (type == SERIAL_HID_TYPE_KEYBOARD) {
                    expected = SERIAL_HID_KEYBOARD_LEN;
                } else {
                    expected = SERIAL_HID_MOUSE_LEN;
                }

                len = b;
                sum ^= b;
                pos = 0;

                if (len == expected) {
                    state = PARSE_PAYLOAD;
                } else {
                    state = PARSE_SOF;
                }
                break;
            }

            case PARSE_PAYLOAD:
                payload[pos++] = b;
                sum ^= b;
                if (pos == len) {
                    state = PARSE_CHECKSUM;
                }
                break;

            case PARSE_CHECKSUM:
                if (b == sum) {
                    serial_listener_dispatch(type, payload);
                } else {
                    ESP_LOGW(TAG, "checksum mismatch: got 0x%02x, want 0x%02x", b, sum);
                }
                state = PARSE_SOF;
                break;
            }
        }
    }
}

esp_err_t serial_listener_init(void)
{
    usb_serial_jtag_driver_config_t cfg = USB_SERIAL_JTAG_DRIVER_CONFIG_DEFAULT();
    cfg.rx_buffer_size = SERIAL_LISTENER_RX_BUF;

    ESP_RETURN_ON_ERROR(usb_serial_jtag_driver_install(&cfg), TAG, "usb serial jtag driver install failed");

    if (xTaskCreate(serial_listener_task, "serial_listener", 3 * 1024, NULL, 5, NULL) != pdPASS) {
        return ESP_ERR_NO_MEM;
    }
    return ESP_OK;
}
