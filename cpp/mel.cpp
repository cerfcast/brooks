#include <functional>
#include <iostream>
#include <string>

using URI_t = std::hash<std::string>;
using HEADERS_t = std::hash<std::string>;

struct Request {
    HEADERS_t h;
    URI_t uri;
    std::string method;
    std::string scheme;
};

INTERPRET_FUNCTION

void test() {
	std::cout << "Test passed.\n";
}

int main() {
	test();
	return 0;
}
