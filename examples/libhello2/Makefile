all: OUTPUT OUTPUT/libhello.a OUTPUT/hello.h

OUTPUT:
	mkdir -p $@

OUTPUT/libhello.a: hello.o
	$(AR) rcs $@ $^

OUTPUT/hello.h: hello.h
	cp -v $^ $@

hello.o: hello.c
	$(CC) -c -o $@ $^

