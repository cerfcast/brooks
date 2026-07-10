#include <functional>
#include <string>

using URI_t = std::hash<std::string>;
using HEADERS_t = std::hash<std::string>;

struct Request {
    HEADERS_t h;
    URI_t uri;
    std::string method;
    std::string scheme;
};

int mel_evaluate(const Request &req) {
    return 1;
}

int main() {

}