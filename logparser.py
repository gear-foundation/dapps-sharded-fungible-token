logfile = open('LOGFILE', 'r')
log_lines = logfile.readlines()

line_counter = 0
malloc_counter = 0
free_counter = 0
realloc_counter = 0

malloc_call_indicator = "MALLOC CALL:"
free_call_indicator = "DL FREE CALL:"
realloc_call_indicator = "DL REALLOC CALL:"

malloc_dict = {}
malloc_keys = []

for line in log_lines:
    line_counter += 1
    if malloc_call_indicator in line:
        malloc_counter += 1
        splited_line = line.split(' ')
        allocated_bytes = int(splited_line.pop(), 16)
        if allocated_bytes in malloc_dict:
            malloc_dict[allocated_bytes] += 1
            continue
        malloc_dict[allocated_bytes] = 1
        continue

    if free_call_indicator in line:
        free_counter += 1
        continue

    if realloc_call_indicator in line:
        realloc_counter += 1
        continue

for key in malloc_dict:
    malloc_keys.append(key)
malloc_keys.sort()

print("==== Malloc statistics dump: block size, amount ====")
for key in malloc_keys:
    print("\t" + str(key) + "\t" + str(malloc_dict[key]))
print("====================================================")

print( "Lines parsed : " + str(line_counter))
print( "Mallocs called : " + str(malloc_counter))
print( "Frees called : " + str(free_counter))
print( "Realloc called : " + str(realloc_counter))
