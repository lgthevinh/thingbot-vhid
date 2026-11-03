set(srcs "ble_hid/ble_hid.c"
         "main.c")
set(include_dirs "." "ble_hid")

idf_component_register(SRCS "${srcs}"
                       INCLUDE_DIRS "${include_dirs}"
                       REQUIRES esp_hid
                       PRIV_REQUIRES nvs_flash bt)
