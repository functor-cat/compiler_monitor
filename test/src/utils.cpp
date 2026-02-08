#include "utils.h"
#include <string.h>

namespace Utils {
    std::string reverse(const std::string& str) {
        std::string result = str;
        int n = result.length();
        for(int i = 0; i < n/2; i++) {
            char temp = result[i];
            result[i] = result[n-i-1];
            result[n-i-1] = temp;
        }
        return result;
    }

    std::string toUpper(const std::string& str) {
        std::string result = str;
        for(size_t i = 0; i < result.length(); i++) {
            if(result[i] >= 'a' && result[i] <= 'z') {
                result[i] = result[i] - 32;
            }
        }
        return result;
    }

    std::string toLower(const std::string& str) {
        std::string result = str;
        for(size_t i = 0; i < result.length(); i++) {
            if(result[i] >= 'A' && result[i] <= 'Z') {
                result[i] = result[i] + 32;
            }
        }
        return result;
    }
}
