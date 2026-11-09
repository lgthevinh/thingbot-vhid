set(srcs "ble_hid/ble_hid.c"
         "serial/serial_listener.c"
         "main.c")
set(include_dirs "." "ble_hid" "serial")

idf_component_register(SRCS "${srcs}"
                       INCLUDE_DIRS "${include_dirs}"
                       REQUIRES esp_hid
                       PRIV_REQUIRES nvs_flash bt esp_driver_usb_serial_jtag)
