#include "DEV_Config.h"
#include "epd_handler.h"
#include "WiFi.h"
#include "constants.h"
#include "ESPAsyncWebServer.h"
#include "server.h"

static WiFiClass wifi;

static QueueHandle_t ipc_queue = xQueueCreate(10, sizeof(EpdJob));

// ReSharper disable CppUseAuto
static WebServer server = WebServer(WEB_SERVER_PORT, ipc_queue);
static EpdHandler epd = EpdHandler(ipc_queue);
// ReSharper restore CppUseAuto

void setup_wifi()
{
    wifi.mode(WIFI_STA);
    wifi.setAutoReconnect(true);
    wifi.begin(WIFI_SSID, WIFI_PASSWORD);
    printf("Begin WIFI connection...\r\n");
    while (wifi.status() != WL_CONNECTED) {
        delay(1000);
        printf(".");
    }
    printf("\r\nConnected to WiFi\r\n");
    printf("IP address: %s\r\n", wifi.localIP().toString().c_str());
    printf("MAC address: %s\r\n", wifi.macAddress().c_str());
    printf("Signal strength: %d dBm\r\n", wifi.RSSI());
    delay(100);
}

void setup()
{
    setup_wifi();


    DEV_Module_Init();

    printf("Starting setup...\r\n");

    server.run();
    epd.start_worker();
}

void loop()
{
}
