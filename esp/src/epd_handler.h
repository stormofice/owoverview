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
    DisplayPartial,

    Undefined,
};

struct EpdJob
{
private:
    EpdJobKind kind;
    uint8_t* data;
    size_t size;
    uint64_t aux[16];

public:
    explicit EpdJob(const EpdJobKind kind) : kind(kind), data(nullptr), size(0), aux{0}
    {
    }

    EpdJob(const EpdJobKind kind, uint8_t* data, const size_t size) : kind(kind), data(data), size(size), aux {0}
    {
    }

    EpdJob(const EpdJobKind kind, uint8_t* data, const size_t size, uint64_t aux[16]) : kind(kind), data(data), size(size) , aux{0}
    {
        memcpy(this->aux, aux, sizeof(this->aux));
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

    uint64_t getAux(const int index) const
    {
        return aux[index];
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
