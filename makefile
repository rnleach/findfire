# Directory layout.
PROJDIR := $(realpath $(CURDIR)/)
SOURCEDIR := $(PROJDIR)/src
MAINSDIR := $(PROJDIR)/mains
TESTDIR := $(PROJDIR)/tests
DOCDIR := $(PROJDIR)/doc

OBJDIR := $(PROJDIR)/obj
BUILDDIR := $(PROJDIR)/build

# Target executable
PROG1 = findfire
PROG2 = connectfire
TEST  = test
TARGET1 = $(BUILDDIR)/$(PROG1)
TARGET2 = $(BUILDDIR)/$(PROG2)
TEST_TARGET = $(BUILDDIR)/$(TEST)

CFLAGS = -g -fPIC -flto -Wall -Werror -O3 -std=c11 -I$(SOURCEDIR)
LDLIBS = -flto -fPIC -lm

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
OBJS := $(subst $(SOURCEDIR), $(OBJDIR), $(SOURCES:.c=.o))
DEPS = $(OBJS:.o=.d)

# Hide or not the calls depending on VERBOSE
ifeq ($(VERBOSE),TRUE)
	HIDE = 
else
	HIDE = @
endif

.PHONY: all clean directories test doc

all: makefile directories $(TARGET1) $(TARGET2)

$(TARGET1): directories makefile $(OBJS) $(MAINSDIR)/$(PROG1).o
	@echo Linking $@
	$(HIDE)$(CC) $(MAINSDIR)/$(PROG1).o $(OBJS) $(LDLIBS) -o $(TARGET1)

$(TARGET2): directories makefile $(OBJS) $(MAINSDIR)/$(PROG2).o
	@echo Linking $@
	$(HIDE)$(CC) $(MAINSDIR)/$(PROG2).o $(OBJS) $(LDLIBS) -o $(TARGET2)

-include $(DEPS)

# Generate rules
$(OBJDIR)/%.o: $(SOURCEDIR)/%.c makefile
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

$(MAINSDIR)/%.o: $(MAINSDIR)/%.c makefile
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

$(TESTDIR)/%.o: $(TESTDIR)/%.c makefile
	@echo Building $@
	$(HIDE)$(CC) -c $(CFLAGS) -o $@ $< -MMD

directories:
	@echo Creating directory $<
	$(HIDE)mkdir -p $(OBJDIR) 2>/dev/null
	$(HIDE)mkdir -p $(BUILDDIR) 2>/dev/null

test: directories makefile $(OBJS) $(TESTDIR)/$(TEST).o
	@echo Linking $@
	$(HIDE)$(CC) $(TESTDIR)/$(TEST).o $(OBJS) $(LDLIBS) -o $(TEST_TARGET)
	$(HIDE) $(TEST_TARGET)

doc: Doxyfile makefile $(SOURCES) $(HEADERS)
	$(HIDE) doxygen 2>/dev/null

clean:
	-$(HIDE)rm -rf $(OBJDIR) $(BUILDDIR) $(DOCDIR) 2>/dev/null
	-$(HIDE)rm $(MAINSDIR)/*.d $(MAINSDIR)/*.o $(TARGET1) $(TARGET2)
	-$(HIDE)rm $(TESTDIR)/*.d $(TESTDIR)/*.o $(TEST_TARGET)
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

