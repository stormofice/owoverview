#include "epd_handler.h"

void EpdHandler::start_worker()
{
    EPD_7IN5_V2_Init();
    EPD_7IN5_V2_Clear();
    EPD_7IN5_V2_Sleep();

    xTaskCreate(
        [](void* raw_queue) {
            const auto queue_handle = static_cast<QueueHandle_t*>(raw_queue);
            printf("[QW] Queue worker started...\r\n");
            EpdJob msg{EpdJobKind::Undefined};

            // Can't mark closure as [[noreturn]]
            // ReSharper disable once CppDFAEndlessLoop
            while (true) {
                if (xQueueReceive(*queue_handle, &msg, portMAX_DELAY)) {
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
                        case EpdJobKind::Init:
                            EPD_7IN5_V2_Init();
                            break;
                        case EpdJobKind::Display: {
                            EPD_7IN5_V2_Init();
                            printf("display task, buf: %p, len: %d\r\n", msg.getData(), msg.getSize());

                            // ensure size match
                            if (msg.getSize() != ((EPD_7IN5_V2_WIDTH / 8) * EPD_7IN5_V2_HEIGHT)) {
                                printf("size mismatch\r\n");
                            }
                            else {
                                printf("size match, sending to epd\r\n");
                                EPD_7IN5_V2_Display(msg.getData());

                                // prevent mem leak
                                delete msg.getData();
                            }
                            EPD_7IN5_V2_Sleep();
                            break;
                        }
                        case EpdJobKind::DisplayPartial: {
                            EPD_7IN5_V2_Init_Part();
                            printf("display partial task, buf: %p, len: %d\r\n", msg.getData(), msg.getSize());
                            const auto x = msg.getAux(0);
                            const auto y = msg.getAux(1);
                            const auto w = msg.getAux(2);
                            const auto h = msg.getAux(3);
                            printf("x: %llu, y: %llu, w: %llu, h: %llu\r\n", x,y,w,h);
                            delay(10);
                            // ensure size match
                            if (msg.getSize() != ((EPD_7IN5_V2_WIDTH / 8) * EPD_7IN5_V2_HEIGHT)) {
                                printf("size mismatch\r\n");
                            }
                            else {
                                printf("size match, sending to epd\r\n");
                                EPD_7IN5_V2_Display_Part(msg.getData(), x, y, x + w, y + h);
                                // prevent mem leak
                                delete msg.getData();
                            }
                            EPD_7IN5_V2_Sleep();

                            break;
                        }
                        case EpdJobKind::Undefined:
                            printf("Undefined job kind, ignoring\r\n");
                            break;
                    }
                }
            }
        },
        "EpdTask",
        2048,
        &this->queue,
        1,
        nullptr);
}
