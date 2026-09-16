/* *********************************************************************
 * This Original Work is copyright of 51 Degrees Mobile Experts Limited.
 * Copyright 2026 51 Degrees Mobile Experts Limited, Davidson House,
 * Forbury Square, Reading, Berkshire, United Kingdom RG1 3EU.
 *
 * This Original Work is licensed under the European Union Public Licence
 * (EUPL) v.1.2 and is subject to its terms as set out below.
 *
 * If a copy of the EUPL was not distributed with this file, You can obtain
 * one at https://opensource.org/licenses/EUPL-1.2.
 *
 * The 'Compatible Licences' set out in the Appendix to the EUPL (as may be
 * amended by the European Commission) shall be deemed incompatible for
 * the purposes of the Work and the provisions of the compatibility
 * clause in Article 5 of the EUPL shall not apply.
 *
 * If using the Work as, or as part of, a network application, by
 * including the attribution notice(s) required under Article 5 of the EUPL
 * in the end user terms of the application under an appropriate heading,
 * such notice(s) shall fulfill the requirements of that article.
 * ********************************************************************* */

#include "resource.h"
#include "fiftyone.h"

/**
 * Macro used to ensure that local variables are aligned to memory boundaries
 * to support interlocked operations that require double width data structures
 * and pointers to be aligned.
 */
#ifndef FIFTYONE_DEGREES_NO_THREADING
#define VOLATILE volatile
#if ((defined(_MSC_VER) && defined(_WIN64)) \
    || (!defined(_MSC_VER) && (defined(_LP64) || defined(__MINGW64__))))
#define ALIGN_SIZE 16
typedef struct counter_t {
	ResourceHandle* handle;
	int32_t inUse;
	int32_t padding;
} Counter;
static InterlockDoubleWidth emptyCounter() {
	InterlockDoubleWidth empty = { 0, 0 };
	((Counter*)&empty)->handle = NULL;
	((Counter*)&empty)->padding = 0;
	((Counter*)&empty)->inUse = 0;
	return empty;
}
#else
#define ALIGN_SIZE 8
typedef struct counter_t {
	ResourceHandle* handle;
	int32_t inUse;
} Counter;
static InterlockDoubleWidth emptyCounter() {
	InterlockDoubleWidth empty = { 0 };
	((Counter*)&empty)->handle = NULL;
	((Counter*)&empty)->inUse = 0;
	return empty;
}
#endif

#ifdef _MSC_VER
// These types' variables in most cases are involved in atomic operations,
// mainly the compare and swap exchange. As it is observed in Windows SDK
// 10.0.18362.0 (VS2017), when being compiled with /Og flag, the optimized code 
// incorrectly update the value held in the destination just prior to the 
// atomic operation of the compare and swap. This appears to be undesired 
// behaviour when optmization is performed on local variables. Thus, to ensure
// that the compare and swap operation is performed correctly, compiler
// optimization should be switch off on the involved variables. To do so
// variables of these types should always be marked with 'volatile' qualifier.
#define COUNTER __declspec(align(ALIGN_SIZE)) volatile InterlockDoubleWidth
#define HANDLE __declspec(align(ALIGN_SIZE)) volatile ResourceHandle
#else
typedef InterlockDoubleWidth AlignedInterlockDoubleWidth
	__attribute__ ((aligned (ALIGN_SIZE)));
#define COUNTER AlignedInterlockDoubleWidth

typedef ResourceHandle AlignedResourceHandle
	__attribute__ ((aligned (ALIGN_SIZE)));
#define HANDLE AlignedResourceHandle
#endif
#else
#define VOLATILE
typedef struct counter_t {
	ResourceHandle* handle;
	int32_t inUse;
} Counter;
static InterlockDoubleWidth emptyCounter() {
	InterlockDoubleWidth empty;
	((Counter*)&empty)->handle = NULL;
	((Counter*)&empty)->inUse = 0;
	return empty;
}
#define COUNTER InterlockDoubleWidth
#define HANDLE ResourceHandle
#endif

static void add(VOLATILE InterlockDoubleWidth* counter, int32_t value) {
	((Counter*)counter)->inUse += value;
}

static int32_t getInUse(VOLATILE InterlockDoubleWidth* counter) {
	return ((Counter*)counter)->inUse;
}

static ResourceHandle* getHandle(VOLATILE InterlockDoubleWidth* counter) {
	return ((Counter*)counter)->handle;
}

static void setHandle(
	VOLATILE InterlockDoubleWidth* counter,
	ResourceHandle* handle) {
	((Counter*)counter)->handle = handle;
}

/**
 * Returns the handle to the resource that is ready to be set for the manager,
 * or NULL if the handle was not successfully created.
 * @param manager of the resource
 * @param resource to be assigned to the manager
 * @parma resourceHandle reference to the handle within the resource
 */
static void setupResource(
	ResourceManager *manager, 
	void *resource, 
	ResourceHandle **resourceHandle,
	void(*freeResource)(void*)) {

	// Needed to verify that the counters size is the same as two pointers.
	assert(sizeof(void*) * 2 == sizeof(InterlockDoubleWidth));

	// Create a new active handle for the manager. Align this to double
	// architecture's bus size to enable double width atomic operations.
	ResourceHandle *handle = (ResourceHandle*)
		MallocAligned(
			sizeof(InterlockDoubleWidth),
			sizeof(ResourceHandle));

	// Set the handle and the number of users of the resource to zero.
	handle->counter = emptyCounter();
	setHandle(&handle->counter, handle);
	
	// Set a link between the new active resource and the manager. Used to
	// check if the resource can be freed when the last thread has finished
	// using it.
	handle->manager = manager;

	// Set the resource that the new handle manages to the one provided.
	handle->resource = resource;

	// Set the free resource method as this may not be available if the manager
	// is disposed of.
	handle->freeResource = freeResource;

	// Ensure the resource's handle is set before assigning the handle
	// as the active handle.
	*resourceHandle = handle;
}

static void freeHandle(volatile ResourceHandle *handle) {
	handle->freeResource((void*)handle->resource);
	FreeAligned((void*)handle);
}

void fiftyoneDegreesResourceManagerInit(
	fiftyoneDegreesResourceManager *manager,
	void *resource,
	fiftyoneDegreesResourceHandle **resourceHandle,
	void(*freeResource)(void*)) {

	// Initialise the manager with the resource ensuring that the resources
	// handle is set before it's made the active resource.
	setupResource(manager, resource, resourceHandle, freeResource);
	manager->active = *resourceHandle;
}

void fiftyoneDegreesResourceManagerFree(
	fiftyoneDegreesResourceManager *manager) {
	// Unlike IncUse and DecUse, Free should not be
	// called at the same time as a reload so the 
	// active handle won't change at this point.
	// Thus, it is safe to perform assertion directly
	// to the active handle here. 
	assert(getInUse(&manager->active->counter) >= 0);
	if (manager->active != NULL) {

		ResourceHandle* newHandlePointer;
		fiftyoneDegreesResourceReplace(
			manager,
			NULL,
			&newHandlePointer);
		FreeAligned(newHandlePointer);
	}
}

void fiftyoneDegreesResourceHandleDecUse(
	fiftyoneDegreesResourceHandle *handle) {
	// When modifying this method, it is important to note the reason for using
	// two separate compareand swaps. The first compare and swap ensures that
	// we are certain the handle is ready to be released i.e. the inUse counter
	// is zero, and the handle is no longer active in the manager. The second
	// compare and swap ensures that we are certain the handle can be freed by
	// THIS thread. See below for an example of when this can happen.
	COUNTER decremented;
#ifndef FIFTYONE_DEGREES_NO_THREADING
	COUNTER compare;
	do {
		compare = handle->counter;
		assert(getInUse(&compare) > 0);
		decremented = compare;
		add(&decremented, -1);
		assert((uintptr_t)&handle->counter % ALIGN_SIZE == 0);
		assert((uintptr_t)&decremented % ALIGN_SIZE == 0);
		assert((uintptr_t)&compare % ALIGN_SIZE == 0);
#ifdef _MSC_VER
// Disable warning against the difference in the use of 'volatile' qualifier.
// Casting won't resolve the issue which is described above with the definitions 
// of COUNTER and HANDLE macros.
#pragma warning (disable: 4090)
#endif
	} while (FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_DW(
		handle->counter, 
		decremented,
		compare) == false);
#ifdef _MSC_VER
#pragma warning (default: 4090)
#endif
#else
	add(&handle->counter, -1);
	decremented = handle->counter;
#endif
	assert(getInUse(&decremented) >= 0);
	if (getInUse(&decremented) == 0 &&  // Am I the last user of the handle?
		handle->manager->active != getHandle(&decremented)) { // Is the handle still active?
#ifndef FIFTYONE_DEGREES_NO_THREADING
		// Atomically set the handle's self pointer to null to ensure only
		// one thread can get into the freeHandle method.
		// Consider the scenario where between the decrement this if statement:
		// 1. another thread increments and decrements the counter, then
		// 2. the active handle is replaced.
		// In this case, both threads will make it into here, so access to
		// the freeHandle method must be limted to one by atomically nulling
		// the self pointer. We will still have access to the pointer for
		// freeing through the decremented copy.
		COUNTER empty = emptyCounter();
		assert((uintptr_t)&getHandle(&decremented)->counter % ALIGN_SIZE == 0);
		assert((uintptr_t)&empty % ALIGN_SIZE == 0);
		assert((uintptr_t)&decremented % ALIGN_SIZE == 0);
#ifdef _MSC_VER
// Disable warning against the difference in the use of 'volatile' qualifier.
// Casting won't resolve the issue which is described above with the definitions 
// of COUNTER and HANDLE macros.
#pragma warning (disable: 4090)
#endif
		if (FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_DW(
			getHandle(&decremented)->counter,
			empty,
			decremented)) {
#ifdef _MSC_VER
#pragma warning (default: 4090)
#endif
			assert(handle != handle->manager->active);
			freeHandle(getHandle(&decremented));
		}
#else
		freeHandle(getHandle(&decremented));
#endif
	}
}

fiftyoneDegreesResourceHandle* fiftyoneDegreesResourceHandleIncUse(
	fiftyoneDegreesResourceManager *manager) {
	COUNTER incremented;
#ifndef FIFTYONE_DEGREES_NO_THREADING
	COUNTER compare;
	do {
		compare = manager->active->counter;
		assert(getInUse(&compare) >= 0);
		incremented = compare;
		add(&incremented, 1);
		assert((uintptr_t)&manager->active->counter % ALIGN_SIZE == 0);
		assert((uintptr_t)&incremented % ALIGN_SIZE == 0);
		assert((uintptr_t)&compare % ALIGN_SIZE == 0);
#ifdef _MSC_VER
// Disable warning against the difference in the use of 'volatile' qualifier.
// Casting won't resolve the issue which is described above with the definitions 
// of COUNTER and HANDLE macros.
#pragma warning (disable: 4090)
#endif
	} while (FIFTYONE_DEGREES_INTERLOCK_EXCHANGE_DW(
		manager->active->counter,
		incremented,
		compare) == false);
#ifdef _MSC_VER
#pragma warning (default: 4090)
#endif
#else
	add(&manager->active->counter, 1);
	incremented = manager->active->counter;
#endif
	assert(getInUse(&incremented) > 0);
	return getHandle(&incremented);
}

int32_t fiftyoneDegreesResourceHandleGetUse(
	fiftyoneDegreesResourceHandle *handle) {
	if (handle != NULL) {
		return getInUse(&handle->counter);
	}
	else {
		return 0;
	}
}

#ifdef _MSC_VER
// Disable warning against the difference in the use of 'volatile' qualifier.
// Casting won't resolve the issue which is described above with the definitions 
// of COUNTER and HANDLE macros.
#pragma warning (disable: 4090)
#endif
void fiftyoneDegreesResourceReplace(
	fiftyoneDegreesResourceManager *manager,
	void *newResource,
	fiftyoneDegreesResourceHandle **newResourceHandle) {
	HANDLE* oldHandle = NULL;
	
	// Add the new resource to the manager replacing the existing one.
	setupResource(
		manager,
		newResource,
		newResourceHandle,
		manager->active->freeResource);
	assert(getInUse(&(*newResourceHandle)->counter) == 0);
	assert(getHandle(&(*newResourceHandle)->counter) == *newResourceHandle);
#ifndef FIFTYONE_DEGREES_NO_THREADING
	// Switch the active handle for the manager to the newly created one.
	do {
		if (oldHandle != NULL) {
			ResourceHandleDecUse(oldHandle);
		}
		oldHandle = ResourceHandleIncUse(manager);
	} while (INTERLOCK_EXCHANGE_PTR(
		manager->active,
		*newResourceHandle,
		oldHandle) == false);
#else
	oldHandle = ResourceHandleIncUse(manager);
	manager->active = *newResourceHandle;
#endif
	// Release the existing resource can be freed. If nothing else is
	// holding onto a reference to it then free it will be freed.
	ResourceHandleDecUse(oldHandle);
}
#ifdef _MSC_VER
#pragma warning (default: 4090)
#endif