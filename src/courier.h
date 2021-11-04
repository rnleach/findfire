#pragma once
/*
 * Copyright (c) 2021 Ryan Leach
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 *
 */

/*
 * Version 1.0.0 - November 3rd, 2021
 */

/** \file courier.h
 * \brief A thread-safe multi-producer / multi-consumer queue in C.

Courier is a sinlge-header source library in C for a threadsafe queue.

It is intended for use cases where there will not be high contention for the queue because the
processing time for each item in the queue will be much, much longer than pushing or popping an
item from the queue. Thread safety is the primary goal.

It is also expected that this queue will only pass pointers since the items are expected to take a
large amount of memory.
 */
#include <assert.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>

#include <pthread.h>

/** The number of elements that the queue can hold.
 *
 * Since this is intended to be used as a source library, you should change this to an appropriate
 * value for your use case.
 */
#define COURIER_QUEUE_SIZE 16

/*
 * Enforce a power of 2 size for the queue. This isn't strictly necessary yet, but it may be
 * necessary in the future.
 */
static_assert((COURIER_QUEUE_SIZE & (COURIER_QUEUE_SIZE - 1)) == 0,
              "COURIER_QUEUE_SIZE must be a power of 2");

/** \brief A threadsafe queue for passing pointers.
 *
 * This should be treated as an opaque type, and only handled through it's functions interface
 * below.
 */
typedef struct Courier {
    size_t head;
    size_t tail;
    size_t count;
    pthread_mutex_t mtx;
    pthread_cond_t space_available;
    pthread_cond_t data_available;
    unsigned int _Atomic num_producers;
    bool _Atomic started;
    void *buf[COURIER_QUEUE_SIZE];
} Courier;

/** \brief Create a new Courier that is initialized and ready to go. */
static inline Courier
courier_new(void)
{
    return (Courier){
        .head = 0,
        .tail = 0,
        .count = 0,
        .mtx = PTHREAD_MUTEX_INITIALIZER,
        .space_available = PTHREAD_COND_INITIALIZER,
        .data_available = PTHREAD_COND_INITIALIZER,
        .num_producers = ATOMIC_VAR_INIT(0),
        .started = ATOMIC_VAR_INIT(false),
        .buf = {0},
    };
}

/** \brief Finalize and destroy a Courier.
 *
 * Since the Courier type can be placed on the stack (recommended), you can no longer use it after
 * calling this function on it, unless you replace it with the result of another call to
 * courier_new().
 */
static inline void
courier_destroy(Courier *cr)
{
    assert(cr);
    assert(cr->num_producers == 0);

    cr->head = 0;
    cr->tail = 0;
    cr->count = 0;

    int rc = pthread_mutex_destroy(&cr->mtx);
    if (rc == EBUSY) {
        fprintf(stderr, "LOGIC ERROR - cannot destroy pthread_mutex_t it is locked!\n");
        assert(!rc);
    }

    rc = pthread_cond_destroy(&cr->space_available);
    if (rc == EBUSY) {
        fprintf(stderr, "LOGIC ERROR - cannot destroy pthread_cond_t it is being waited on!\n");
        assert(!rc);
    }

    rc = pthread_cond_destroy(&cr->data_available);
    if (rc == EBUSY) {
        fprintf(stderr, "LOGIC ERROR - cannot destroy pthread_cond_t it is being waited on!\n");
        assert(!rc);
    }

    cr->started = false;
    memset(cr->buf, 0, sizeof(cr->buf));
}

/** Blocks until the Courier is ready to pass data to a receiver. */
static inline void
courier_wait_until_ready(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);
    while (!cr->started) {
        pthread_cond_wait(&cr->data_available, &cr->mtx);
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}

/** Open a Courier for sending data. */
static inline void
courier_open(Courier *cr)
{
    assert(cr);

    cr->num_producers += 1;
    cr->started = true;
}

/** Close a Courier for sending data. */
static inline void
courier_close(Courier *cr)
{
    assert(cr);
    assert(cr->num_producers > 0);

    cr->num_producers -= 1;

    // Broadcast in case anyone is waiting for data to come available that won't!
    pthread_cond_broadcast(&cr->data_available);
}

/** Push a value onto the queue.
 *
 * If all the calls to courier_open() have been matched with calls to courier_close(), then it is
 * assumed the courier is no longer accepting data, and calling this function will abort the
 * program.
 *
 * Otherwise, this will add \a data to the queue unless it is full, in which case it will block
 * until there is space available.
 */
static inline void
courier_send(Courier *cr, void *data)
{
    assert(cr);

    if (cr->num_producers == 0) {
        fprintf(stderr, "LOGIC ERROR - courier channel closed, no producers, cannot send.\n");
        exit(EXIT_FAILURE);
    }

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);
    while (cr->count == COURIER_QUEUE_SIZE) {
        pthread_cond_wait(&cr->space_available, &cr->mtx);
    }

    cr->buf[(cr->tail) % COURIER_QUEUE_SIZE] = data;
    cr->tail += 1;
    cr->count += 1;

    pthread_cond_broadcast(&cr->data_available);
    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}

/** Retrieve a value from the queue.
 *
 * If all the calls to courier_open() have been matched with calls to courier_close(), then it is
 * assumed the courier is no longer accepting data, and calling this function will continue to
 * return values until the queue is empty. Once the queue is empty, this will return \c NULL.
 *
 * Otherwise, this will return the next value in the queue or block until one is available.
 */
static inline void *
courier_receive(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);
    while (cr->count == 0 && cr->num_producers > 0) {
        pthread_cond_wait(&cr->data_available, &cr->mtx);
    }

    void *return_val = 0;
    if (cr->count > 0) {

        return_val = cr->buf[(cr->head) % COURIER_QUEUE_SIZE];
        cr->head += 1;
        cr->count -= 1;
    }

    pthread_cond_broadcast(&cr->space_available);
    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);

    return return_val;
}
