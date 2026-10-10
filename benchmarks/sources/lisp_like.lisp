; A recursive list operation with strings, numbers, and booleans.
(define (map-values function values)
  (if (equal? values '())
      '()
      (cons (function (first values))
            (map-values function (rest values)))))

(define values '(1 2 3 4 5 -6))
(define enabled #t)
(define message "mapped values")
(map-values (lambda (value) (* value value)) values)

(define (fold-left function accumulator values)
  (if (equal? values '())
      accumulator
      (fold-left function
                 (function accumulator (first values))
                 (rest values))))

(define (filter-values predicate values)
  (if (equal? values '())
      '()
      (if (predicate (first values))
          (cons (first values) (filter-values predicate (rest values)))
          (filter-values predicate (rest values)))))

(define (factorial value)
  (if (<= value 1)
      1
      (* value (factorial (- value 1)))))

(define (fibonacci value)
  (if (< value 2)
      value
      (+ (fibonacci (- value 1))
         (fibonacci (- value 2)))))

(define names '(alpha beta gamma delta epsilon))
(define numbers '(13 -8 21 34 55 89 144))
(define total (fold-left + 0 numbers))
(define positives (filter-values (lambda (value) (> value 0)) numbers))
(define report (list "total" total "enabled" enabled "names" names))

(if #f
    (display "disabled branch")
    (display "active branch"))

(map-values factorial '(1 2 3 4 5 6))
(map-values fibonacci '(3 4 5 6 7 8))
