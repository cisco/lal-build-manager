#include <stdio.h>

void hello(char *name) {
  if (name == NULL) {
    printf("Hello World!\n");
  } else {
    printf("Hello %s!\n", name);
  }
}
