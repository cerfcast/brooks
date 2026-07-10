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

bool interpret() {
	// 0 to 9
	std::string _var_0 = "testing";
	// 13 to 17
	std::regex _var_1 = std::regex(".*");
	// 0 to 17
	bool _var_2 = std::regex_match(_var_0, _var_1);
	// 0 to 17
	return _var_2;
}


void test() {
	std::cout << "Test passed.\n";
}

int main() {
	test();
	return 0;
}
