#pragma once

#include "ESPAsyncWebServer.h"
#include <memory>

class WebServer
{
    QueueHandle_t job_queue;
    std::unique_ptr<AsyncWebServer> server;


    class PartialUpload
    {
    public:
        explicit PartialUpload(uint8_t* data) : data(data), acc_size(0)
        {
        }

        uint8_t* data;
        size_t acc_size;
    };

public:
    explicit WebServer(const uint16_t port, QueueHandle_t job_queue): job_queue(job_queue),
                                                                      server(std::make_unique<AsyncWebServer>(port))
    {
    };

    void run() const;
};
