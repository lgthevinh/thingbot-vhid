#include "esp_err.h"
#include "nvs_flash.h"

#include "ble_hid.h"
#include "serial_listener.h"

void app_main(void)
{
    // NimBLE stores bonding keys in NVS; erase and retry if the partition is full or from an older layout
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_ERROR_CHECK(ret);

    ESP_ERROR_CHECK(ble_hid_init("ThingBot"));
    ESP_ERROR_CHECK(serial_listener_init());
}
