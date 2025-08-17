// Simple startup code that calls main and exits properly
extern int main();

// For ARM64 Linux, exit system call number is 93
void _start() {
    int result = main();
    
    // Exit system call: syscall number 93, parameter in x0
    asm volatile (
        "mov x8, #93\n"     // System call number for exit
        "mov x0, %0\n"      // Exit code
        "svc #0\n"          // System call
        :
        : "r" (result)
        : "x0", "x8"
    );
    
    // Should never reach here
    while(1);
}
