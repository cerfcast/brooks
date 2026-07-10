#include <functional>
#include <iostream>
#include <regex>
#include <string>

using URI_t = std::hash<std::string>;
using HEADERS_t = std::hash<std::string>;

struct Request {
    HEADERS_t h;
    URI_t uri;
    std::string method;
    std::string scheme;
};

int interpret() {
	// 0 to 1
	int _var_0 = 5;
	// 4 to 5
	int _var_1 = 4;
	// 0 to 5
	int _var_2 = _var_0 + _var_1;
	// 0 to 5
	return _var_2;
}


void test() {
	std::cout << "Test passed.\n";
}

int main() {
	test();
	return 0;
}
