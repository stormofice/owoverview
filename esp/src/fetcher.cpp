#include "fetcher.h"

EpdJob Fetcher::fetch()
{
    // Do it messy for now

    WiFiClient wifiClient;
    HttpClient client{wifiClient, "1.1.1.1", 6765};

}

