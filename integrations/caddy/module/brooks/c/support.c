#include "brooks.h"

int caddy_brooks_result_user_indirect(caddy_brooks_result_user called, uintptr_t data, uintptr_t cookie) {
	return called((void*)data, (void*)cookie);
}
int caddy_brooks_result_user_indirect2(caddy_brooks_result_user2 called, uintptr_t data, uintptr_t data2, uintptr_t cookie) {
	return called((void*)data, (void*)data2, (void*)cookie);
}
