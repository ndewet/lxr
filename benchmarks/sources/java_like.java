public class Calculator {
    public int baseValue = 10;
    public boolean enabled = true;

    /* This method exercises arithmetic and conditional branches. */
    public static int calculate(int value) {
        int result = value * 42 + 3.5;
        boolean ready = result >= 0 && value != 0;
        if (ready) {
            return result;
        } else {
            return 0;
        }
    }

    public static int clamp(int value, int minimum, int maximum) {
        if (value < minimum) {
            return minimum;
        } else if (value > maximum) {
            return maximum;
        }
        return value;
    }

    public static boolean acceptable(int value) {
        boolean positive = value > 0;
        boolean bounded = value <= 1000;
        return positive && bounded || value == 42;
    }

    public static void process(int limit) {
        int current = 0;
        while (current <= limit) {
            Calculator calculator = new Calculator();
            int candidate = calculator.calculate(current);
            int normalized = clamp(candidate, 0, 500);
            if (acceptable(normalized)) {
                current = current + 1;
            } else {
                current = current + 2;
            }
        }
    }

    public static int select(boolean enabled, int primary, int fallback) {
        if (enabled == true) {
            return primary;
        }
        if (enabled == false) {
            return fallback;
        }
        return 0;
    }

    public static void main() {
        Calculator first = new Calculator();
        Calculator second = new Calculator();
        int alpha = first.calculate(17);
        int beta = second.calculate(99);
        int selected = select(alpha >= beta, alpha, beta);
        String message = "selected value";
        Object absent = null;
        process(selected);
    }
}
