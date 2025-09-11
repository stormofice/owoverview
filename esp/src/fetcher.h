#pragma once
#include "epd_handler.h"
#include <ArduinoHttpClient.h>
#include "WiFi.h"

class Fetcher
{
    HttpClient client;
public:
    EpdJob fetch();
    void init(WiFiClient wifi);
};
