CFLAGS=-static -IINPUT
LDFLAGS=-LINPUT/libhello
LIBS=-lhello

all: OUTPUT OUTPUT/hello
OUTPUT/hello: main.c
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^ $(LIBS)

OUTPUT:
	mkdir -p OUTPUT
