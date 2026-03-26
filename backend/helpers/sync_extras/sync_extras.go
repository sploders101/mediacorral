package sync_extras

import (
	"context"
	"errors"
	"sync"
	"sync/atomic"
)

var (
	ErrImmutable = errors.New("watched value is no longer mutable")
)

type Watch[T any] struct {
	mu         sync.Mutex
	cond       *sync.Cond
	value      T
	generation uint64
	finished   bool
}

// Creates a new `Watch` instance
func NewWatch[T any](initial T) *Watch[T] {
	w := &Watch[T]{
		value: initial,
		// Start generation at 1 so new watchers always get a value
		generation: 1,
	}
	w.cond = sync.NewCond(&w.mu)
	return w
}

// Creates a new `Watch` instance, but waits until the first update before notifying receivers
func NewWatchSilent[T any](initial T) *Watch[T] {
	w := &Watch[T]{
		value:      initial,
		generation: 0,
	}
	w.cond = sync.NewCond(&w.mu)
	return w
}

// Gets the current value
//
// Useful for point-in-time requests.
func (w *Watch[T]) Get() T {
	w.mu.Lock()
	defer w.mu.Unlock()
	return w.value
}

// Gets the current value, but leaves the mutex locked.
//
// This function returns the value and a function to unlock the mutex.
// The unlock function ***MUST*** be called, or the watcher will be permanently deadlocked. The unlock function may be
// called multiple times with no adverse effects.
//
// This is useful for pointer types like `map`.
func (w *Watch[T]) GetLocked() (value T, unlock func()) {
	w.mu.Lock()
	var unlocked atomic.Bool
	return w.value, func() {
		if !unlocked.Swap(true) {
			w.mu.Unlock()
		}
	}
}

// Sets a new value
//
// This will wake any downstream listeners and alert them to the new value.
func (w *Watch[T]) Set(v T) {
	w.mu.Lock()
	if w.finished {
		w.mu.Unlock()
		return
	}
	w.value = v
	w.generation++
	w.mu.Unlock()
	w.cond.Broadcast()
}

// Conditionally sets a new value
//
// This will call the provided function, allowing it to modify the inner value, and use the returned boolean to
// determine if watchers should be notified.
//
// NOTE: If the watch has been closed, the function will not be executed.
func (w *Watch[T]) ConditionalSet(setFunc func(*T) bool) {
	w.mu.Lock()
	if w.finished {
		w.mu.Unlock()
		return
	}
	updated := setFunc(&w.value)
	if updated {
		w.generation++
	}
	w.mu.Unlock()
	if updated {
		w.cond.Broadcast()
	}
}

// Close the watcher
//
// This will signal that the watcher will not receive any more values. Updates will be ignored.
func (w *Watch[T]) Close() {
	w.mu.Lock()
	defer w.mu.Unlock()
	w.finished = true
	w.cond.Broadcast()
}

// Subscribes to this watcher, returning a handle that can be used to subscibe to changes
func (w *Watch[T]) Watch() WatchReceiver[T] {
	return WatchReceiver[T]{
		context:    context.Background(),
		watcher:    w,
		generation: 0,
	}
}

// Subscribes to this watcher, returning a handle that can be used to subscibe to changes
func (w *Watch[T]) WatchLocked() WatchReceiverLocked[T] {
	return WatchReceiverLocked[T]{
		context:    context.Background(),
		watcher:    w,
		generation: 0,
	}
}

// Waits for a relevant change on the watcher, returning a copy of the value, the generation, and whether or not the
// watcher has been closed
func (w *Watch[T]) wait(
	ctx context.Context,
	prevVersion uint64,
	unlock bool,
) (T, uint64, bool, error) {
	w.mu.Lock()
	if unlock {
		defer w.mu.Unlock()
	}

	valueChan := make(chan struct{}, 1)
	go func() {
		select {
		case <-valueChan:
		case <-ctx.Done():
			w.cond.Broadcast()
		}
	}()

	for w.generation == prevVersion && !w.finished && ctx.Err() == nil {
		w.cond.Wait()
	}

	valueChan <- struct{}{}

	return w.value, w.generation, w.finished, nil
}

// A handle for watching changes to a `Watcher`'s value.
type WatchReceiver[T any] struct {
	context    context.Context
	watcher    *Watch[T]
	generation uint64
}

// Sets the context for the receiver to use when watching for updates
func (receiver *WatchReceiver[T]) SetContext(ctx context.Context) {
	receiver.context = ctx
}

// Clones the receiver so it can be watched concurrently. New values will go to both receivers
func (receiver *WatchReceiver[T]) Clone() WatchReceiver[T] {
	return WatchReceiver[T]{
		watcher:    receiver.watcher,
		generation: receiver.generation,
	}
}

// Gets the current value from the receiver.
func (receiver *WatchReceiver[T]) Get() T {
	return receiver.watcher.Get()
}

// Waits for the watched value to change.
//
// If the watcher has been marked immutable (ie. it will never change again), and the last value has already been seen,
// this function returns `ErrImmutable`.
//
// If the context given to the receiver expires, this function returns `context.Cancelled`.
func (receiver *WatchReceiver[T]) Changed() (T, error) {
	val, gen, finished, err := receiver.watcher.wait(receiver.context, receiver.generation, true)
	if err != nil {
		return val, err
	}
	if finished && receiver.generation == gen {
		return val, ErrImmutable
	}
	receiver.generation = gen
	return val, nil
}

// A handle for watching changes to a `Watcher`'s value.
//
// Leaves the mutex locked when returning so the value remains safe to use.
// Failure to unlock the value will create a deadlock.
type WatchReceiverLocked[T any] struct {
	context    context.Context
	watcher    *Watch[T]
	generation uint64
}

// Sets the context for the receiver to use when watching for updates
func (receiver *WatchReceiverLocked[T]) SetContext(ctx context.Context) {
	receiver.context = ctx
}

// Clones the receiver so it can be watched concurrently. New values will go to both receivers
func (receiver *WatchReceiverLocked[T]) Clone() WatchReceiverLocked[T] {
	return WatchReceiverLocked[T]{
		watcher:    receiver.watcher,
		generation: receiver.generation,
	}
}

// Gets the current value from the receiver.
//
// Returns `T`, `unlock`. Take care to call `unlock` as quickly as possible to allow other functions to use the value.
// It is safe to call `unlock` more than once.
func (receiver *WatchReceiverLocked[T]) Get() (T, func()) {
	return receiver.watcher.GetLocked()
}

// Waits for the watched value to change.
//
// This function leaves the value locked and returns `value`, `unlock`, `error`. Failure to call the unlock function
// will result in a deadlock. No other receiver can see the value until `unlock` is called, so keep usage as short as
// possible. It is safe to call the unlock function multiple times.
//
// If the watcher has been marked immutable (ie. it will never change again), and the last value has already been seen,
// this function returns `ErrImmutable`.
//
// If the context given to the receiver expires, this function returns `context.Cancelled`.
func (receiver *WatchReceiverLocked[T]) Changed() (T, func(), error) {
	val, gen, finished, err := receiver.watcher.wait(receiver.context, receiver.generation, false)
	if err != nil {
		receiver.watcher.mu.Unlock()
		return val, func() {}, err
	}
	if finished && receiver.generation == gen {
		receiver.watcher.mu.Unlock()
		return val, func() {}, ErrImmutable
	}
	receiver.generation = gen
	var unlocked atomic.Bool
	return val, func() {
		if !unlocked.Swap(true) {
			receiver.watcher.mu.Unlock()
		}
	}, nil
}
