#pragma once
#include "epd_handler.h"
#include <ArduinoHttpClient.h>
#include "WiFi.h"

class Fetcher
{
public:
    EpdJob fetch();
};
