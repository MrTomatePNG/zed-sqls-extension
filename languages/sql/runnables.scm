; Detecta SELECT statements
(select_statement) @run

; Detecta INSERT statements
(insert_statement) @run

; Detecta UPDATE statements
(update_statement) @run

; Detecta DELETE statements
(delete_statement) @run

; Detecta CREATE statements
(create_statement) @run

; Tag para identificar como SQL query
((select_statement) @run
 (#set! tag sql-query))

((insert_statement) @run
 (#set! tag sql-query))

((update_statement) @run
 (#set! tag sql-query))

((delete_statement) @run
 (#set! tag sql-query))

((create_statement) @run
 (#set! tag sql-query))
