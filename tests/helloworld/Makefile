all: hello.o hello

hello.o: hello.c
	$(CC) -IINPUT/heylib -O -c hello.c

hello: hello.o INPUT/heylib/libhey.a INPUT/heylib/hey.h
	$(CC) --static -o OUTPUT/hello hello.o -LINPUT/heylib -lhey

clean:
	rm hello.o
