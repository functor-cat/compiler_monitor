#define _USE_MATH_DEFINES
#include <stdio.h>
#include <assert.h>
#include "mathops.h"
#include "utils.h"
#include "calculator.h"

int main() {
    printf("=== Compiler Monitor Test Suite ===\n\n");
    
    int passed = 0;
    int failed = 0;
    
    // Test Math operations
    printf("Testing Math namespace functions...\n");
    
    // Test add
    if (Math::add(5, 3) == 8) {
        printf("  [PASS] Math::add(5, 3) = 8\n");
        passed++;
    } else {
        printf("  [FAIL] Math::add(5, 3) != 8\n");
        failed++;
    }
    
    if (Math::add(-5, 3) == -2) {
        printf("  [PASS] Math::add(-5, 3) = -2\n");
        passed++;
    } else {
        printf("  [FAIL] Math::add(-5, 3) != -2\n");
        failed++;
    }
    
    // Test subtract
    if (Math::subtract(10, 3) == 7) {
        printf("  [PASS] Math::subtract(10, 3) = 7\n");
        passed++;
    } else {
        printf("  [FAIL] Math::subtract(10, 3) != 7\n");
        failed++;
    }
    
    // Test multiply
    if (Math::multiply(4, 5) == 20) {
        printf("  [PASS] Math::multiply(4, 5) = 20\n");
        passed++;
    } else {
        printf("  [FAIL] Math::multiply(4, 5) != 20\n");
        failed++;
    }
    
    // Test divide
    double divResult = Math::divide(10, 2);
    if (divResult == 5.0) {
        printf("  [PASS] Math::divide(10, 2) = 5.0\n");
        passed++;
    } else {
        printf("  [FAIL] Math::divide(10, 2) != 5.0\n");
        failed++;
    }
    
    // Test divide by zero
    if (Math::divide(10, 0) == 0.0) {
        printf("  [PASS] Math::divide(10, 0) returns 0.0 (safe)\n");
        passed++;
    } else {
        printf("  [FAIL] Math::divide(10, 0) doesn't handle zero safely\n");
        failed++;
    }
    
    // Test Calculator class
    printf("\nTesting Calculator class...\n");
    
    Calculator calc;
    
    // Test initial state
    if (calc.getResult() == 0.0) {
        printf("  [PASS] Calculator initialized to 0.0\n");
        passed++;
    } else {
        printf("  [FAIL] Calculator not initialized to 0.0\n");
        failed++;
    }
    
    // Test add
    calc.add(10.5);
    if (calc.getResult() == 10.5) {
        printf("  [PASS] Calculator::add(10.5) = 10.5\n");
        passed++;
    } else {
        printf("  [FAIL] Calculator::add(10.5) != 10.5\n");
        failed++;
    }
    
    // Test subtract
    calc.subtract(5.5);
    if (calc.getResult() == 5.0) {
        printf("  [PASS] Calculator::subtract(5.5) = 5.0\n");
        passed++;
    } else {
        printf("  [FAIL] Calculator::subtract(5.5) != 5.0\n");
        failed++;
    }
    
    // Test multiply
    calc.multiply(2.0);
    if (calc.getResult() == 10.0) {
        printf("  [PASS] Calculator::multiply(2.0) = 10.0\n");
        passed++;
    } else {
        printf("  [FAIL] Calculator::multiply(2.0) != 10.0\n");
        failed++;
    }
    
    // Test divide
    calc.divide(5.0);
    if (calc.getResult() == 2.0) {
        printf("  [PASS] Calculator::divide(5.0) = 2.0\n");
        passed++;
    } else {
        printf("  [FAIL] Calculator::divide(5.0) != 2.0\n");
        failed++;
    }
    
    // Test clear
    calc.clear();
    if (calc.getResult() == 0.0) {
        printf("  [PASS] Calculator::clear() resets to 0.0\n");
        passed++;
    } else {
        printf("  [FAIL] Calculator::clear() doesn't reset to 0.0\n");
        failed++;
    }
    
    // Test Utils functions
    printf("\nTesting Utils namespace functions...\n");
    
    // Test reverse
    if (Utils::reverse("hello") == "olleh") {
        printf("  [PASS] Utils::reverse(\"hello\") = \"olleh\"\n");
        passed++;
    } else {
        printf("  [FAIL] Utils::reverse(\"hello\") != \"olleh\"\n");
        failed++;
    }
    
    // Test toUpper
    if (Utils::toUpper("hello") == "HELLO") {
        printf("  [PASS] Utils::toUpper(\"hello\") = \"HELLO\"\n");
        passed++;
    } else {
        printf("  [FAIL] Utils::toUpper(\"hello\") != \"HELLO\"\n");
        failed++;
    }
    
    // Test toLower
    if (Utils::toLower("WORLD") == "world") {
        printf("  [PASS] Utils::toLower(\"WORLD\") = \"world\"\n");
        passed++;
    } else {
        printf("  [FAIL] Utils::toLower(\"WORLD\") != \"world\"\n");
        failed++;
    }
    
    // Print summary
    printf("\n=== Test Summary ===\n");
    printf("Total tests: %d\n", passed + failed);
    printf("Passed: %d\n", passed);
    printf("Failed: %d\n", failed);
    
    if (failed == 0) {
        printf("\n[SUCCESS] All tests passed!\n");
        return 0;
    } else {
        printf("\n[FAILURE] Some tests failed!\n");
        return 1;
    }
}
