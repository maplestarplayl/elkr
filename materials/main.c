// Declare `sum` as an external function. The definition is elsewhere.
extern int sum(int a, int b);

// A global variable.
int my_global = 10;

int main() {
    int result = sum(3, 19);
    return result;
}