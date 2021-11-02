# Directory layout.
PROJDIR := $(realpath $(CURDIR)/)
SOURCEDIR := $(PROJDIR)/src
MAINSDIR := $(PROJDIR)/mains
TESTDIR := $(PROJDIR)/tests
DOCDIR := $(PROJDIR)/doc

OBJDIR := $(PROJDIR)/obj
RELEASEDIR := $(PROJDIR)/release
DEBUGDIR := $(PROJDIR)/debug

# Target executable
PROG1 = findfire
PROG2 = connectfire
TEST  = test
TARGET1 = $(BUILDDIR)/$(PROG1)
TARGET2 = $(BUILDDIR)/$(PROG2)
TEST_TARGET = $(BUILDDIR)/$(TEST)

CFLAGS = -Wall -Werror -std=c11 -I$(SOURCEDIR)
LDLIBS = -lm
ifeq ($(DEBUG),1)
	CFLAGS += -g
	LDLIBS +=
	BUILDDIR := $(DEBUGDIR)
else
	CFLAGS += -DNDEBUG -fPIC -flto -O3 
	LDLIBS += -flto -fPIC
	BUILDDIR := $(RELEASEDIR)
endif

# -------------------------------------------------------------------------------------------------
# enable some time functions for POSIX
CFLAGS += -D_DEFAULT_SOURCE -D_XOPEN_SOURCE -D_GNU_SOURCE

# glib
CFLAGS += `pkg-config --cflags glib-2.0`
LDLIBS += `pkg-config --libs glib-2.0`

# gdal
CFLAGS += `pkg-config --cflags gdal`
LDLIBS += `pkg-config --libs gdal`

# sqlite3 library for download cache
CFLAGS += `pkg-config --cflags sqlite3`
LDLIBS += `pkg-config --libs sqlite3`
# -------------------------------------------------------------------------------------------------

# Compiler and compiler options
CC = clang

# Show commands make uses
VERBOSE = TRUE

# Add this list to the VPATH, the place make will look for the source files
VPATH = $(SOURCEDIR)

# Create a list of *.c files in DIRS
SOURCES = $(wildcard $(SOURCEDIR)/*.c)
HEADERS = $(wildcard $(SOURCEDIR)/*.h)

# Define object files for all sources, and dependencies for all objects
OBJS := $(subst $(SOURCEDIR), $(BUILDDIR), $(SOURCES:.c=.o))
DEPS = $(OBJS:.o=.d)

# Hide or not the calls depending on VERBOSE
ifeq ($(VERBOSE),TRUE)
	HIDE = 
else
	HIDE = @
endif

.PHONY: all clean directories test doc

all: makefile directories $(TARGET1) $(TARGET2) $(TEST_TARGET)

$(TARGET1): directories makefile $(OBJS) $(BUILDDIR)/$(PROG1).o
	@echo Linking $@
	$(HIDE)$(CC) $(BUILDDIR)/$(PROG1).o $(OBJS) $(LDLIBS) -o $@

$(BUILDDIR)/$(PROG1).o: $(MAINSDIR)/$(PROG1).c
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

$(TARGET2): directories makefile $(OBJS) $(BUILDDIR)/$(PROG2).o
	@echo Linking $@
	$(HIDE)$(CC) $(BUILDDIR)/$(PROG2).o $(OBJS) $(LDLIBS) -o $@

$(BUILDDIR)/$(PROG2).o: $(MAINSDIR)/$(PROG2).c
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

$(TEST_TARGET): directories makefile $(OBJS) $(BUILDDIR)/$(TEST).o
	@echo Linking $@
	$(HIDE)$(CC) $(BUILDDIR)/$(TEST).o $(OBJS) $(LDLIBS) -o $@

$(BUILDDIR)/$(TEST).o: $(TESTDIR)/$(TEST).c
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

-include $(DEPS)

# Generate rules
$(BUILDDIR)/%.o: $(SOURCEDIR)/%.c makefile
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

directories:
	@echo Creating directory $<
	$(HIDE)mkdir -p $(BUILDDIR) 2>/dev/null

test: directories makefile $(OBJS) $(BUILDDIR)/$(TEST).o
	@echo Linking $@
	$(HIDE)$(CC) $(BUILDDIR)/$(TEST).o $(OBJS) $(LDLIBS) -o $(TEST_TARGET)
	$(HIDE) $(TEST_TARGET)

doc: Doxyfile makefile $(SOURCES) $(HEADERS)
	$(HIDE) doxygen 2>/dev/null

clean:
	-$(HIDE)rm -rf $(DEBUGDIR) $(RELEASEDIR) $(DOCDIR) 2>/dev/null
	@echo Cleaning done!

#detected_OS = $(shell uname)
#ifeq ($(detected_OS), Linux)
#	target_dir = ~/usr/bin/
#else
#	target_dir = ~/bin/
#endif
#
#install: $(TARGET1) makefile
#	strip $(TARGET1) && cp $(TARGET1) $(target_dir)

