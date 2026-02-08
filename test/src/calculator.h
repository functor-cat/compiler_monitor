#pragma once

class Calculator {
private:
    double result;

public:
    Calculator();
    void add(double value);
    void subtract(double value);
    void multiply(double value);
    void divide(double value);
    void clear();
    double getResult() const;
};
