#include "DEV_Config.h"
#include "epd_handler.h"
#include "WiFi.h"
#include "constants.h"
#include "fetcher.h"

static QueueHandle_t ipc_queue = xQueueCreate(10, sizeof(EpdJob));

// ReSharper disable CppUseAuto
static EpdHandler epd = EpdHandler(ipc_queue);
static Fetcher fetcher = Fetcher{};
// ReSharper restore CppUseAuto

void setup_wifi()
{

    WiFi.begin(WIFI_SSID, WIFI_PASSWORD);
    printf("Begin WIFI connection...\r\n");
    while (WiFi.status() != WL_CONNECTED) {
        delay(1000);
        printf(".");
    }
    printf("\r\nConnected to WiFi\r\n");
    printf("IP address: %s\r\n", WiFi.localIP().toString().c_str());
    printf("MAC address: %s\r\n", WiFi.macAddress().c_str());
    printf("Signal strength: %d dBm\r\n", WiFi.RSSI());
    delay(100);
}

void setup()
{
    setup_wifi();

    DEV_Module_Init();

    printf("Starting setup...\r\n");

    epd.start_worker();

}

void loop()
{
    fetcher.fetch();
}
