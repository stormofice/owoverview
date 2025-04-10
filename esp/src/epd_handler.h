#pragma once
#include <cstdint>
#include <freertos/FreeRTOS.h>
#include <freertos/queue.h>
#include <freertos/task.h>
#include "EPD.h"

enum class EpdJobKind
{
    Clear,
    ClearBlack,
    Sleep,
    Display,
    Init,

    Undefined,
};

struct EpdJob
{
private:
    EpdJobKind kind;
    uint8_t* data;
    size_t size;

public:
    explicit EpdJob(const EpdJobKind kind) : kind(kind), data(nullptr), size(0)
    {
    }

    EpdJob(const EpdJobKind kind, uint8_t* data, const size_t size) : kind(kind), data(data), size(size)
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
};

class EpdHandler
{
    QueueHandle_t queue;

public:
    explicit EpdHandler(QueueHandle_t queue) : queue(queue)
    {
    }

    void start_worker();
};
