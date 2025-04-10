#include "DEV_Config.h"
#include "EPD.h"
#include "WiFi.h"
#include "constants.h"
#include "ESPAsyncWebServer.h"
#include "GUI_Paint.h"


enum class EpdJobKind;
static WiFiClass wifi;

static AsyncWebServer server(WEB_SERVER_PORT);

static QueueHandle_t epd_task_queue = nullptr;

void setup_wifi()
{
    wifi.mode(WIFI_STA);
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

enum class EpdJobKind
{
    Clear,
    ClearBlack,
    Sleep,
    Display,

    Undefined,
};

struct EpdJob
{
private:
    EpdJobKind kind;
    uint8_t* data;
    size_t size;

public:
    size_t actual_length;

    explicit EpdJob(const EpdJobKind kind) : kind(kind), data(nullptr), size(0)
    {
    }

    EpdJob(const EpdJobKind kind, uint8_t* data, const size_t size) : kind(kind), data(data), size(size),
                                                                      actual_length(0)
    {
    }

    EpdJobKind getKind() const
    {
        return kind;
    }

    uint8_t* getData() const
    {
        return data;
    }

    size_t getSize() const
    {
        return size;
    }

    void setSize(const size_t size)
    {
        this->size = size;
    }
};


void setup_server()
{
    server.on("/", HTTP_GET, [](AsyncWebServerRequest* request) {
        request->send(200, "text/plain", "ello! :3");
    });

    server.on("/clear", HTTP_GET, [](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::Clear};
        xQueueSendToBack(epd_task_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Clearing display...");
    });

    server.on("/clear_black", HTTP_GET, [](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::ClearBlack};
        xQueueSendToBack(epd_task_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Clearing display black...");
    });


    // TODO: make safe
    server.on("/upload_image", HTTP_POST, [](AsyncWebServerRequest* request) {
                  if (!request->_tempObject) {
                      return request->send(400, "text/plain", "Invalid request, nothing uploaded");
                  }

                  printf("Upload end\r\n");

                  const auto job = static_cast<EpdJob *>(request->_tempObject);
                  job->setSize(job->actual_length);
                  xQueueSendToBack(epd_task_queue, request->_tempObject, portMAX_DELAY);

                  request->_tempObject = nullptr;
                  request->send(200, "text/plain", "Upload successful");
              }, [](AsyncWebServerRequest* request, const String&filename, const size_t index, const uint8_t* data,
                    const size_t len,
                    bool _) {
                  if (!index) {
                      // First pass
                      printf("Upload start: %s\n", filename.c_str());

                      // ~~ 1152000 = 800 * 480 * 3
                      constexpr auto max_upload_size = 1152000 + 256;
                      const auto file_length = request->header("Content-Length").toInt();
                      if (file_length > max_upload_size) {
                          request->send(400, "text/plain", "File too large");
                          return;
                      }

                      // Allocate memory for the image
                      const auto file_data = new uint8_t[file_length];
                      const auto job = new EpdJob{EpdJobKind::Display, file_data, file_length};
                      request->_tempObject = job;
                  }

                  if (len) {
                      const auto job = static_cast<EpdJob *>(request->_tempObject);
                      memcpy(job->getData() + index, data, len);
                      job->actual_length += len;
                  }
              });


    server.on("/sleep", HTTP_GET, [](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::Sleep};
        xQueueSendToBack(epd_task_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Sleeping display...");
    });

    server.begin();
    printf("Server started on port %d\r\n", WEB_SERVER_PORT);
    delay(100);
}

void calm_down_display()
{
    printf("[EPD] Init and Clear...\r\n");
    EPD_7IN5_V2_Init();
    EPD_7IN5_V2_Clear();

    printf("[EPD] Sleep...\r\n");
    EPD_7IN5_V2_Sleep();
}

void setup_ipc()
{
    epd_task_queue = xQueueCreate(10, sizeof(EpdJob));
}

void setup()
{
    DEV_Module_Init();

    printf("Starting setup...\r\n");

    setup_ipc();

    setup_wifi();

    printf("Starting web server...\r\n");
    setup_server();

    printf("Initializing EPD display...\r\n");
    EPD_7IN5_V2_Init();
    delay(200);

    // Start queue worker
    xTaskCreate(
        [](void* _) {
            printf("[QW] Queue worker started...\r\n");
            EpdJob msg{EpdJobKind::Undefined};
            constexpr auto imagesize = ((EPD_7IN5_V2_WIDTH % 8 == 0)
                                            ? (EPD_7IN5_V2_WIDTH / 8)
                                            : (EPD_7IN5_V2_WIDTH / 8 + 1)) * EPD_7IN5_V2_HEIGHT;
            auto* image = static_cast<uint8_t *>(calloc(imagesize, 1));
            Paint_NewImage(image, EPD_7IN5_V2_WIDTH, EPD_7IN5_V2_HEIGHT, 0, WHITE);
            Paint_SelectImage(image);

            // Can't mark closure as [[noreturn]]
            // ReSharper disable once CppDFAEndlessLoop
            while (true) {
                if (xQueueReceive(epd_task_queue, &msg, portMAX_DELAY)) {
                    printf("Received message: %d\r\n", msg.getKind());
                    // Process the message
                    switch (msg.getKind()) {
                        case EpdJobKind::Clear:
                            EPD_7IN5_V2_Clear();
                            break;
                        case EpdJobKind::ClearBlack:
                            EPD_7IN5_V2_ClearBlack();
                            break;
                        case EpdJobKind::Sleep:
                            EPD_7IN5_V2_Sleep();
                            break;
                        case EpdJobKind::Display:
                            printf("display task, buf: %p, len: %d\r\n", msg.getData(), msg.getSize());
                            // ensure size match
                            if (msg.getSize() != ((EPD_7IN5_V2_WIDTH/8) * EPD_7IN5_V2_HEIGHT)) {
                                printf("size mismatch\r\n");
                            } else {
                                printf("size match, sending to epd\r\n");
                                EPD_7IN5_V2_Display(msg.getData());

                                // prevent mem leak
                                delete msg.getData();
                            }
                            break;
                        case EpdJobKind::Undefined:
                            printf("Undefined job kind, ignoring\r\n");
                            break;
                    }
                }
            }
        },
        "EpdTask",
        2048,
        nullptr,
        1,
        nullptr);
}

void loop()
{
}
