#include "calculator.h"

Calculator::Calculator() : result(0.0) {}

void Calculator::add(double value) {
    result += value;
}

void Calculator::subtract(double value) {
    result -= value;
}

void Calculator::multiply(double value) {
    result *= value;
}

void Calculator::divide(double value) {
    if (value != 0.0) {
        result /= value;
    }
}

void Calculator::clear() {
    result = 0.0;
}

double Calculator::getResult() const {
    return result;
}
