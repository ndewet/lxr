// A varied source exercises keywords, identifiers, literals, and punctuation.
struct Calculator {
    value: Integer,
    label: String,
}

enum Operation {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl Calculator {
    fn new(initial_value) -> Calculator {
        return Calculator { value: initial_value, label: "primary" };
    }

    fn calculate(value_123, operation) -> Integer {
        let scaled = value_123 * 42 + 3.5;
        if scaled == 0 {
            return "zero";
        } else {
            return "nonzero";
        }
    }

    fn normalize(numerator, denominator) {
        if denominator != 0 {
            return numerator / denominator;
        }
        return 0;
    }
}

fn process(limit) {
    let current = 0;
    let total = 0;
    while current <= limit {
        for item in current {
            let next_value = item + 1;
            total = total + Calculator.calculate(next_value, Operation.Add);
        }
    }
    return total;
}

fn classify(value) {
    if value >= 1000 {
        return "large";
    } else if value >= 10 {
        return "medium";
    } else {
        return "small";
    }
}

fn main() {
    let calculator = Calculator.new(25);
    let first_result = calculator.calculate(125, Operation.Multiply);
    let second_result = calculator.normalize(first_result, 5);
    let category = classify(second_result);
    return category;
}
