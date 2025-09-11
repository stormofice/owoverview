#include "fetcher.h"

uint32_t bytes_to_u32_le(uint8_t b0, uint8_t b1, uint8_t b2, uint8_t b3)
{
    return static_cast<uint32_t>(b0) |
           (static_cast<uint32_t>(b1) << 8) |
           (static_cast<uint32_t>(b2) << 16) |
           (static_cast<uint32_t>(b3) << 24);
}

EpdJob Fetcher::fetch()
{
    // Do it messy for now
    printf("start fetch\n");

    WiFiClient wifiClient;
    HttpClient client{wifiClient, "192.168.178.35", 7676};

    client.setHttpResponseTimeout(30000);

    auto err = client.get("/image");
    if (err != 0) {
        printf("Error while trying to fetch image: %d\n", err);
        return EpdJob{EpdJobKind::Clear};
    }

    const auto statusCode = client.responseStatusCode();
    printf("Status Code: %d\n", statusCode);
    
    // cL http parsing seems broken somehow
    long contentLength = -1;
    while (client.headerAvailable()) {
        String headerName = client.readHeaderName();
        String headerValue = client.readHeaderValue();
        if (headerName.equalsIgnoreCase("Content-Length")) {
            contentLength = headerValue.toInt();
        }
    }
    
    printf("Content Length: %ld\n", contentLength);

    if (contentLength == -1) {
        printf("No content length");
        return EpdJob{EpdJobKind::Clear};
    }

    auto* buf = new uint8_t[contentLength];

    size_t totalRead = 0;
    while (totalRead < contentLength) {
        int bytesRead = client.read(buf + totalRead, contentLength - totalRead);
        if (bytesRead <= 0) {
            printf("have to wait for data?...\n");
            delay(20);
            continue;
        }
        totalRead += bytesRead;
        printf("read %d bytes (total: %zu of %ld)\n", bytesRead, totalRead, contentLength);
    }

    auto command = bytes_to_u32_le(buf[0], buf[1], buf[2], buf[3]);
    if (command == 0x0) {
        // Full update
        printf("Full update command received\r\n");

        const auto fixed_data = new uint8_t[contentLength - 4];
        memcpy(fixed_data, buf + 4, static_cast<size_t>(contentLength) - 4);
        delete[] buf;

        return EpdJob{EpdJobKind::Display, fixed_data, static_cast<size_t>(contentLength) - 4};
    }
    else if (command == 0x1) {
        // Partial
        printf("Partial update command received\r\n");
        // cba
        uint64_t aux[4] = {0};
        aux[0] = bytes_to_u32_le(buf[4], buf[5], buf[6], buf[7]);
        aux[1] = bytes_to_u32_le(buf[8], buf[9], buf[10], buf[11]);
        aux[2] = bytes_to_u32_le(buf[12], buf[13], buf[14], buf[15]);
        aux[3] = bytes_to_u32_le(buf[16], buf[17], buf[18], buf[19]);

        const auto fixed_data = new uint8_t[contentLength - 20];
        memcpy(fixed_data, buf + 20, static_cast<size_t>(contentLength) - 20);
        delete[] buf;

        return EpdJob{EpdJobKind::DisplayPartial, fixed_data, static_cast<size_t>(contentLength) - 20, aux};
    }
    else {
        printf("Unknown image command\r\n");
        return EpdJob{EpdJobKind::Clear};
    }
}
