#include "server.h"
#include <epd_handler.h>

void WebServer::run() const
{
    server->on("/", HTTP_GET, [this](AsyncWebServerRequest* request) {
        request->send(200, "text/plain", "ello! :3");
    });

    server->on("/init", HTTP_GET, [this](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::Init};
        xQueueSendToBack(this->job_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Initializing display...");
    });

    server->on("/clear", HTTP_GET, [this](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::Clear};
        xQueueSendToBack(this->job_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Clearing display...");
    });

    server->on("/clear_black", HTTP_GET, [this](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::ClearBlack};
        xQueueSendToBack(this->job_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Clearing display black...");
    });


    // @formatter:off
    server->on("/upload_image", HTTP_POST, [this](AsyncWebServerRequest* request) {
        if (!request->_tempObject) {
            return request->send(400, "text/plain", "Invalid request, nothing uploaded");
        }

        printf("Upload end\r\n");

        const auto partial_upload = static_cast<PartialUpload *>(request->_tempObject);
        const auto job = EpdJob {EpdJobKind::Display, partial_upload->data, partial_upload->acc_size};

        xQueueSendToBack(this->job_queue, &job, portMAX_DELAY);

        request->_tempObject = nullptr;
        request->send(200, "text/plain", "Upload successful");
    }, [this](AsyncWebServerRequest* request, const String&filename, const size_t index, const uint8_t* data, const size_t len, bool _) {
        if (!index) {
            // First pass
            printf("Upload start: %s\n", filename.c_str());

            // ~~ 1152000 = 800 * 480 * 3 + a bit of space for the request
            constexpr auto max_upload_size = 1152000 + 256;
            const auto file_length = static_cast<size_t>(request->header("Content-Length").toInt());
            if (file_length > max_upload_size) {
                request->send(400, "text/plain", "File too large");
                return;
            }

            // Allocate memory for the image
            const auto file_data = new uint8_t[file_length];
            const auto partial_upload = new PartialUpload{file_data};
            request->_tempObject = partial_upload;
        }

        if (len) {
           const auto partial_upload = static_cast<PartialUpload *>(request->_tempObject);
           memcpy(partial_upload->data + index, data, len);
           partial_upload->acc_size += len;
        }
    });
    // @formatter:on

    server->on("/sleep", HTTP_GET, [this](AsyncWebServerRequest* request) {
        const auto job = EpdJob{EpdJobKind::Sleep};
        xQueueSendToBack(this->job_queue, &job, portMAX_DELAY);
        request->send(200, "text/plain", "Sleeping display...");
    });

    printf("Starting server...\r\n");
    server->begin();
}
