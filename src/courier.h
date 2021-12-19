#pragma once
#pragma clang diagnostic ignored "-Wunknown-warning-option"
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
 * Version 2.0.5 - December 18th, 2021
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
    unsigned int num_producers;
    unsigned int num_consumers;
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
        .num_producers = 0,
        .num_consumers = 0,
        .buf = {0},
    };
}

/** \brief Finalize and destroy a Courier.
 *
 * Since the Courier type can be placed on the stack (recommended), you can no longer use it after
 * calling this function on it, unless you replace it with the result of another call to
 * courier_new().
 *
 * This function is NOT threadsafe. It should only be called to clean up a Courier after all
 * producers and consumers have been deregistered.
 */
static inline void
courier_destroy(Courier *cr, void (*free_func)(void *))
{
    assert(cr);
    assert(cr->num_producers == 0);
    assert(cr->num_consumers == 0);

    if (free_func) {
        while (cr->count > 0) {
            free_func(cr->buf[cr->head % COURIER_QUEUE_SIZE]);
            cr->head += 1;
            cr->count -= 1;
        }

        assert(cr->head == cr->tail && cr->count == 0);
    }

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

    memset(cr->buf, 0, sizeof(cr->buf));
}

// Make clang-13 happy when -Wall and -Werror are used.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
/** Blocks until the Courier is ready to pass data to a receiver. */
static inline void
courier_wait_until_ready_to_receive(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);
    while (cr->num_producers == 0 && cr->count == 0) {
        pthread_cond_wait(&cr->data_available, &cr->mtx);
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}
#pragma clang diagnostic pop

// Make clang-13 happy when -Wall and -Werror are used.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
/** Blocks until the Courier is ready to accept data from a consumer. */
static inline void
courier_wait_until_ready_to_send(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);
    while (cr->num_consumers == 0) {
        pthread_cond_wait(&cr->space_available, &cr->mtx);
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}
#pragma clang diagnostic pop

/** Register a Courier for sending data. */
static inline void
courier_register_sender(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);

    cr->num_producers += 1;

    if (cr->num_producers == 1) {
        // Broadcast here so any threads blocked in courier_wait_until_ready_to_receive() can
        // progress. If the num_producers is greater than 1, then this signal was already sent.
        rc = pthread_cond_broadcast(&cr->data_available);
        if (rc) {
            fputs("Error broadcasting data_available after registering sender!\n", stderr);
        }
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}

/** Register a Courier for receiving data. */
static inline void
courier_register_receiver(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);

    cr->num_consumers += 1;

    if (cr->num_consumers == 1) {
        // Broadcast here so any threads blocked in courier_wait_until_ready_to_send() can progress.
        // If num_consumers > 1, then this message was already sent.
        rc = pthread_cond_broadcast(&cr->space_available);

        if (rc) {
            fputs("Error broadcasting space_available after registering receiver!\n", stderr);
        }
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}

/** Close a Courier for sending data. */
static inline void
courier_done_sending(Courier *cr)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);
    assert(cr->num_producers > 0);

    cr->num_producers -= 1;

    if (cr->num_producers == 0) {
        // Broadcast in case anyone is waiting for data to come available that won't!
        // This will let them check the num_producers variable and realize nothing is coming.
        rc = pthread_cond_broadcast(&cr->data_available);

        if (rc) {
            fputs("Error broadcasting data_available after deregistering sender!\n", stderr);
        }

    } else {
        // Deadlock may occur if I don't do this. This thread got signaled when others should
        // have.
        rc = pthread_cond_broadcast(&cr->space_available);

        if (rc) {
            fputs("Error broadcasting space_available after deregistering sender!\n", stderr);
        }
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}

/** Close a Courier for receiving data. */
static inline void
courier_done_receiving(Courier *cr)
{
    assert(cr);
    assert(cr->num_consumers > 0);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);

    cr->num_consumers -= 1;

    if (cr->num_consumers == 0) {
        // Broadcast in case anyone is waiting to send data that they never will be able too!
        // This will let them check the num_consumers variable and realize space will never become
        // available.
        rc = pthread_cond_broadcast(&cr->space_available);

        if (rc) {
            fputs("Error broadcasting space_available after deregistering receiver!\n", stderr);
        }
    } else {
        // Deadlock may occur if I don't do this. This thread got signaled when others should
        // have.
        rc = pthread_cond_broadcast(&cr->data_available);

        if (rc) {
            fputs("Error broadcasting space_available after deregistering sender!\n", stderr);
        }
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);
}

// Make clang-13 happy when -Wall and -Werror are used.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
/** Push a value onto the queue.
 *
 * If all the calls to courier_register_sender() have been matched with calls to
 * courier_done_sending(), then it is assumed the courier is no longer accepting data, and calling
 * this function will abort the program.
 *
 * Otherwise, this will add \a data to the queue unless it is full, in which case it will block
 * until there is space available or all the receivers are deregistered.
 *
 * \returns \c true on success. If their are no receivers registered with the Courier, then this
 * will return \c false.
 */
static inline bool
courier_send(Courier *cr, void *data)
{
    assert(cr);

    int rc = pthread_mutex_lock(&cr->mtx);
    assert(!rc);

    if (cr->num_producers == 0) {
        fprintf(stderr, "LOGIC ERROR - courier channel closed, no producers, cannot send.\n");
        exit(EXIT_FAILURE);
    }

    while (cr->count == COURIER_QUEUE_SIZE && cr->num_consumers > 0) {
        pthread_cond_wait(&cr->space_available, &cr->mtx);
    }

    if (cr->num_consumers == 0) {
        // Space will never come available again, so FAIL!
        rc = pthread_mutex_unlock(&cr->mtx);
        assert(!rc);
        return false;
    }

    cr->buf[(cr->tail) % COURIER_QUEUE_SIZE] = data;
    cr->tail += 1;
    cr->count += 1;

    // If the count was increased to 1, then someone may have been waiting to be notified!
    if (cr->count == 1) {
        pthread_cond_signal(&cr->data_available);
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);

    return true;
}
#pragma clang diagnostic pop

// Make clang-13 happy when -Wall and -Werror are used.
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wunused-but-set-variable"
/** Retrieve a value from the queue.
 *
 * If all the calls to courier_register_sender() have been matched with calls to
 * courier_done_sending(), then it is assumed the courier is no longer accepting data, and calling
 * this function will continue to return values until the queue is empty. Once the queue is empty,
 * this will return \c NULL. If there is no data available in the queue, this will block until
 * something becomes available or a thread calls courier_done_sending().
 *
 * \returns a pointer. If the pointer is \c NULL, then the queue is empty and there are no more
 * senders registered on it.
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

    // If the queue was full before, we should send a signal to let others know it's got space
    // available now.
    if (cr->count + 1 == COURIER_QUEUE_SIZE) {
        pthread_cond_signal(&cr->space_available);
    }

    rc = pthread_mutex_unlock(&cr->mtx);
    assert(!rc);

    return return_val;
}
#pragma clang diagnostic pop
