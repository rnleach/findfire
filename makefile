# Directory layout.
PROJDIR := $(realpath $(CURDIR)/)
SOURCEDIR := $(PROJDIR)/src
OBJDIR := $(PROJDIR)/obj
BUILDDIR := $(PROJDIR)/build
MAINSDIR := $(PROJDIR)/mains

# Target executable
PROG1 = findfire
PROG2 = connectfire
TARGET1 = $(BUILDDIR)/$(PROG1)
TARGET2 = $(BUILDDIR)/$(PROG2)
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

# Define object files for all sources, and dependencies for all objects
OBJS := $(subst $(SOURCEDIR), $(OBJDIR), $(SOURCES:.c=.o))
MAIN_OBJS := $(subst $(MAINSDIR), $(MAINSDIR), $(SOURCES:.c=.o))
DEPS = $(OBJS:.o=.d)

# Hide or not the calls depending on VERBOSE
ifeq ($(VERBOSE),TRUE)
	HIDE = 
else
	HIDE = @
endif

.PHONY: all clean directories 

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

directories:
	@echo Creating directory $<
	$(HIDE)mkdir -p $(OBJDIR) 2>/dev/null
	$(HIDE)mkdir -p $(BUILDDIR) 2>/dev/null

clean:
	$(HIDE)rm -rf $(OBJDIR) $(BUILDDIR) 2>/dev/null
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

