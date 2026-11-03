#include "ble_hid.h"

#include <string.h>

#include "esp_hidd.h"
#include "esp_check.h"
#include "esp_log.h"
#include "host/ble_hs.h"
#include "nimble/nimble_port.h"
#include "nimble/nimble_port_freertos.h"
#include "services/gap/ble_svc_gap.h"

#define BLE_HID_SVC_UUID16 0x1812

// provided by NimBLE's store/config but not declared in any public header
void ble_store_config_init(void);

static const char *TAG = "ble_hid";

static const uint8_t ble_hid_report_map[] = {
    // Keyboard: input 8 bytes, output 1 byte (LEDs)
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x06, // Usage (Keyboard)
    0xA1, 0x01, // Collection (Application)
    0x85, BLE_HID_REPORT_ID_KEYBOARD, //   Report ID (1)

    // byte 0: modifier bits (LCtrl, LShift, LAlt, LGUI, RCtrl, RShift, RAlt, RGUI)
    0x05, 0x07, //   Usage Page (Keyboard/Keypad)
    0x19, 0xE0, //   Usage Minimum (Left Control)
    0x29, 0xE7, //   Usage Maximum (Right GUI)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x01, //   Logical Maximum (1)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x08, //   Report Count (8)
    0x81, 0x02, //   Input (Data, Var, Abs)

    // byte 1: reserved
    0x75, 0x08, //   Report Size (8)
    0x95, 0x01, //   Report Count (1)
    0x81, 0x01, //   Input (Const)

    // output byte: LED bits (Num, Caps, Scroll, Compose, Kana) + 3 bits padding
    0x05, 0x08, //   Usage Page (LEDs)
    0x19, 0x01, //   Usage Minimum (Num Lock)
    0x29, 0x05, //   Usage Maximum (Kana)
    0x75, 0x01, //   Report Size (1)
    0x95, 0x05, //   Report Count (5)
    0x91, 0x02, //   Output (Data, Var, Abs)
    0x75, 0x03, //   Report Size (3)
    0x95, 0x01, //   Report Count (1)
    0x91, 0x01, //   Output (Const)

    // bytes 2-7: up to 6 pressed keycodes
    0x05, 0x07, //   Usage Page (Keyboard/Keypad)
    0x19, 0x00, //   Usage Minimum (0)
    0x29, 0x65, //   Usage Maximum (0x65)
    0x15, 0x00, //   Logical Minimum (0)
    0x25, 0x65, //   Logical Maximum (0x65)
    0x75, 0x08, //   Report Size (8)
    0x95, 0x06, //   Report Count (6)
    0x81, 0x00, //   Input (Data, Array, Abs)
    0xC0,       // End Collection

    // Mouse: input 4 bytes
    0x05, 0x01, // Usage Page (Generic Desktop)
    0x09, 0x02, // Usage (Mouse)
    0xA1, 0x01, // Collection (Application)
    0x85, BLE_HID_REPORT_ID_MOUSE, //   Report ID (2)
    0x09, 0x01, //   Usage (Pointer)
    0xA1, 0x00, //   Collection (Physical)

    // byte 0: buttons (left, right, middle) + 5 bits padding
    0x05, 0x09, //     Usage Page (Button)
    0x19, 0x01, //     Usage Minimum (Button 1)
    0x29, 0x03, //     Usage Maximum (Button 3)
    0x15, 0x00, //     Logical Minimum (0)
    0x25, 0x01, //     Logical Maximum (1)
    0x75, 0x01, //     Report Size (1)
    0x95, 0x03, //     Report Count (3)
    0x81, 0x02, //     Input (Data, Var, Abs)
    0x75, 0x05, //     Report Size (5)
    0x95, 0x01, //     Report Count (1)
    0x81, 0x01, //     Input (Const)

    // bytes 1-3: relative X, Y, wheel
    0x05, 0x01, //     Usage Page (Generic Desktop)
    0x09, 0x30, //     Usage (X)
    0x09, 0x31, //     Usage (Y)
    0x09, 0x38, //     Usage (Wheel)
    0x15, 0x81, //     Logical Minimum (-127)
    0x25, 0x7F, //     Logical Maximum (127)
    0x75, 0x08, //     Report Size (8)
    0x95, 0x03, //     Report Count (3)
    0x81, 0x06, //     Input (Data, Var, Rel)
    0xC0,       //   End Collection
    0xC0,       // End Collection
};

static esp_hid_raw_report_map_t report_maps[] = {
    { .data = ble_hid_report_map, .len = sizeof(ble_hid_report_map) },
};

static esp_hidd_dev_t *ble_hid_dev;
static const char *ble_hid_name;

/*
 * GAP callback for the connection created from our advertising.
 * Needed to accept re-pairing: when the host has removed the device but we still
 * hold its old bond, NimBLE rejects pairing unless we drop that bond here.
 */
static int ble_hid_gap_event(struct ble_gap_event *event, void *arg)
{
    if (event->type == BLE_GAP_EVENT_REPEAT_PAIRING) {
        struct ble_gap_conn_desc desc;
        if (ble_gap_conn_find(event->repeat_pairing.conn_handle, &desc) == 0) {
            ble_store_util_delete_peer(&desc.peer_id_addr);
        }
        return BLE_GAP_REPEAT_PAIRING_RETRY;
    }
    return 0;
}

/*
 * Advertise as a connectable HID keyboard until a host connects.
 * Called on host sync and after every disconnect, since advertising stops on connect.
 */
static void ble_hid_adv_start(void)
{
    ble_uuid16_t hid_uuid = BLE_UUID16_INIT(BLE_HID_SVC_UUID16);
    struct ble_hs_adv_fields fields = {
        .flags = BLE_HS_ADV_F_DISC_GEN | BLE_HS_ADV_F_BREDR_UNSUP,
        .name = (const uint8_t *)ble_hid_name,
        .name_len = strlen(ble_hid_name),
        .name_is_complete = 1,
        .appearance = ESP_HID_APPEARANCE_KEYBOARD,
        .appearance_is_present = 1,
        .uuids16 = &hid_uuid,
        .num_uuids16 = 1,
        .uuids16_is_complete = 1,
    };
    struct ble_gap_adv_params params = {
        .conn_mode = BLE_GAP_CONN_MODE_UND,
        .disc_mode = BLE_GAP_DISC_MODE_GEN,
    };

    int rc = ble_gap_adv_set_fields(&fields);
    if (rc == 0) {
        rc = ble_gap_adv_start(BLE_OWN_ADDR_PUBLIC, NULL, BLE_HS_FOREVER, &params, ble_hid_gap_event, NULL);
    }
    if (rc != 0) {
        ESP_LOGE(TAG, "adv start failed: %d", rc);
    }
}

// Runs the NimBLE host event loop; returns only after nimble_port_stop()
static void ble_hid_host_task(void *param)
{
    nimble_port_run();
    nimble_port_freertos_deinit();
}

static void ble_hid_event_cb(void *handler_args, esp_event_base_t base, int32_t id, void *event_data)
{
    esp_hidd_event_data_t *param = (esp_hidd_event_data_t *)event_data;

    switch ((esp_hidd_event_t)id) 
    {
    case ESP_HIDD_START_EVENT:
        ESP_LOGI(TAG, "start");
        ble_hid_adv_start();
        break;
    case ESP_HIDD_CONNECT_EVENT:
        ESP_LOGI(TAG, "connect");
        break;
    case ESP_HIDD_PROTOCOL_MODE_EVENT:
        ESP_LOGI(TAG, "protocol mode: %s", param->protocol_mode.protocol_mode ? "report" : "boot");
        break;
    case ESP_HIDD_CONTROL_EVENT:
        ESP_LOGI(TAG, "control: %s", param->control.control ? "exit suspend" : "suspend");
        break;
    case ESP_HIDD_OUTPUT_EVENT:
        ESP_LOGI(TAG, "output id %u, len %u", param->output.report_id, (unsigned)param->output.length);
        ESP_LOG_BUFFER_HEX(TAG, param->output.data, param->output.length);
        break;
    case ESP_HIDD_FEATURE_EVENT:
        ESP_LOGI(TAG, "feature id %u, len %u", param->feature.report_id, (unsigned)param->feature.length);
        break;
    case ESP_HIDD_DISCONNECT_EVENT:
        ESP_LOGI(TAG, "disconnect: %s",
                 esp_hid_disconnect_reason_str(ESP_HID_TRANSPORT_BLE, param->disconnect.reason));
        ble_hid_adv_start();
        break;
    case ESP_HIDD_STOP_EVENT:
        ESP_LOGI(TAG, "stop");
        break;
    default:
        break;
    }
}

/*
 * Bring up NimBLE and register the HID device.
 * Advertising starts from ble_hid_event_cb once the host has synced.
 * Requires NVS to be initialised first (bonding keys are stored there).
 */
esp_err_t ble_hid_init(const char *device_name)
{
    static esp_hid_device_config_t config = {
        // 0x16C0/0x05DF is the shared V-USB test VID/PID; replace for a real product
        .vendor_id = 0x16C0,
        .product_id = 0x05DF,
        .version = 0x0100,
        .manufacturer_name = "ThingEdu",
        .serial_number = "0001",
        .report_maps = report_maps,
        .report_maps_len = 1,
    };
    config.device_name = device_name;
    ble_hid_name = device_name;

    // starts the BT controller and the NimBLE host (host task is started last, below)
    ESP_RETURN_ON_ERROR(nimble_port_init(), TAG, "nimble init");

    // no-PIN ("Just Works") pairing with bonding, so the host reconnects without re-pairing
    ble_hs_cfg.sm_io_cap = BLE_SM_IO_CAP_NO_IO;
    ble_hs_cfg.sm_bonding = 1;
    ble_hs_cfg.sm_sc = 1;
    ble_hs_cfg.sm_our_key_dist = BLE_SM_PAIR_KEY_DIST_ENC | BLE_SM_PAIR_KEY_DIST_ID;
    ble_hs_cfg.sm_their_key_dist = BLE_SM_PAIR_KEY_DIST_ENC | BLE_SM_PAIR_KEY_DIST_ID;
    ble_hs_cfg.store_status_cb = ble_store_util_status_rr;
    ble_store_config_init();

    // registers the HID GATT services; NimBLE fixes its GATT table at sync, so this must precede the host task
    ESP_RETURN_ON_ERROR(esp_hidd_dev_init(&config, ESP_HID_TRANSPORT_BLE, ble_hid_event_cb, &ble_hid_dev),
                        TAG, "hidd init");

    // name and appearance shown by the host after connecting (GAP service, set up by esp_hidd_dev_init)
    ble_svc_gap_device_name_set(device_name);
    ble_svc_gap_device_appearance_set(ESP_HID_APPEARANCE_KEYBOARD);

    nimble_port_freertos_init(ble_hid_host_task);
    return ESP_OK;
}

bool ble_hid_is_connected(void)
{
    return ble_hid_dev && esp_hidd_dev_connected(ble_hid_dev);
}

esp_err_t ble_hid_send_keyboard(const ble_hid_keyboard_report_t *report)
{
    // esp_hidd_dev_input_set() takes a non-const pointer but only copies the data
    return esp_hidd_dev_input_set(
        ble_hid_dev, 0, 
        BLE_HID_REPORT_ID_KEYBOARD,
        (uint8_t *)report, 
        sizeof(*report)
    );
}

esp_err_t ble_hid_send_mouse(const ble_hid_mouse_report_t *report)
{
    return esp_hidd_dev_input_set(
        ble_hid_dev, 
        0, 
        BLE_HID_REPORT_ID_MOUSE,
        (uint8_t *)report, 
        sizeof(*report)
    );
}
