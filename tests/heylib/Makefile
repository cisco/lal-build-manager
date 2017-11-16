all: output

hey.o: hey.c hey.h
	$(CC) -O -c hey.c

libhey.a: hey.o
	ar rcs libhey.a hey.o

output: libhey.a hey.h
	cp libhey.a OUTPUT/
	cp hey.h OUTPUT/

clean:
	rm hey.o libhey.a
