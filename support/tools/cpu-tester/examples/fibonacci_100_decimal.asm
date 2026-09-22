; Fibonacci sequence decimal output test program.
; Output format: ASCII decimal values, one per line.
; Arithmetic and decimal conversion are both performed by the guest.
;
; Representation:
;   - Each Fibonacci value is stored as 24 base-10 digits in little-endian order.
;   - a[0] is the least-significant decimal digit.
;   - The program prints F(0) through F(99).

LI r1, 0x2000      ; DebugIo base
LI r2, 0x2010      ; TestReporter base
LI r3, 100         ; remaining values
LI r4, 0x1000      ; a digits base
LI r5, 0x1020      ; b digits base
LI r6, 0x1040      ; temp digits base
LI r7, 24          ; digits per value

LI r8, 1
STB r8, r5, r0, 0  ; b = 1

main_loop:
BZ r3, done
CALL print_number
CALL add_big

ADD r10, r5, r0, 0 ; copy b -> a
ADD r11, r4, r0, 0
ADD r9, r7, r0, 0
CALL copy_bytes

ADD r10, r6, r0, 0 ; copy temp -> b
ADD r11, r5, r0, 0
ADD r9, r7, r0, 0
CALL copy_bytes

ADD r3, r3, r0, -1
JMP main_loop

done:
LI r8, 1
STW r8, r2, r0, 0

print_number:
ADD r8, r7, r0, -1 ; idx = digits - 1

print_scan_loop:
LDB r13, r4, r8, 0
BZ r13, print_scan_zero
JMP print_scan_found

print_scan_zero:
BZ r8, print_scan_found
ADD r8, r8, r0, -1
JMP print_scan_loop

print_scan_found:
ADD r9, r4, r8, 0  ; ptr = a + idx
ADD r10, r8, r0, 1 ; count = idx + 1

print_emit_loop:
BZ r10, print_newline
LDB r11, r9, r0, 0
ADD r12, r11, r0, 48
STB r12, r1, r0, 0
ADD r9, r9, r0, -1
ADD r10, r10, r0, -1
JMP print_emit_loop

print_newline:
LI r12, 10
STB r12, r1, r0, 0
RET

add_big:
ADD r9, r7, r0, 0  ; count
ADD r10, r4, r0, 0 ; a ptr
ADD r11, r5, r0, 0 ; b ptr
ADD r12, r6, r0, 0 ; temp ptr
LI r14, 0          ; carry

add_loop:
BZ r9, add_done
LDB r8, r10, r0, 0
LDB r13, r11, r0, 0
ADD r8, r8, r13, 0
ADD r8, r8, r14, 0
ADD r13, r8, r0, -10
SHIFT r14, r13, r0, 0xC01F
BZ r14, add_with_carry

STB r8, r12, r0, 0
LI r14, 0
JMP add_advance

add_with_carry:
STB r13, r12, r0, 0
LI r14, 1

add_advance:
ADD r10, r10, r0, 1
ADD r11, r11, r0, 1
ADD r12, r12, r0, 1
ADD r9, r9, r0, -1
JMP add_loop

add_done:
BZ r14, add_ok
LI r8, 1
STW r8, r2, r0, 8
LI r8, 2
STW r8, r2, r0, 0

overflow_halt:
JMP overflow_halt

add_ok:
RET

copy_bytes:
copy_loop:
BZ r9, copy_done
LDB r8, r10, r0, 0
STB r8, r11, r0, 0
ADD r10, r10, r0, 1
ADD r11, r11, r0, 1
ADD r9, r9, r0, -1
JMP copy_loop

copy_done:
RET
