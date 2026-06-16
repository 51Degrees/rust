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

#ifndef FIFTYONE_DEGREES_MANAGER_INCLUDED
#define FIFTYONE_DEGREES_MANAGER_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesResource Resource Manager
 *
 * Resources to be managed by a resource manager.
 *
 * ## Terms
 * 
 * **Resource** : a pointer to anything which needs to be replaced or freed in
 * a thread-safe manor
 *
 * **Manager** : manages access to a resource by handing out handles and
 * altering the resource safely
 *
 * **Handle** : a reference to a resource which is being managed. This is the
 * only way in which a resource should be accessed
 *
 * ## Introduction
 *
 * A Resource is a structure which can be changed or freed at any time, so
 * is managed by a resource manager to allow safe concurrent access. Any
 * resource which is being used will not be changed or freed until the last
 * reference to it has been released.
 *
 * ## Create
 *
 * A resource manager is created by allocating the memory for the structure and
 * calling the #fiftyoneDegreesResourceManagerInit method to give it a resource
 * to manage, and a method to free it when necessary.
 *
 * ## Get
 *
 * A handle to a resource can be fetched from a resource manager using the
 * #fiftyoneDegreesResourceHandleIncUse method. This increments the "in use"
 * counter in a thread safe manor and returns an exclusive handle to the
 * resource.
 *
 * When getting a handle to a resource, if all the available handles are in use
 * then the method will block until one is available.
 *
 * ## Release
 *
 * Once a resource handle is finished with, it must be released back to the
 * resource manager using the #fiftyoneDegreesResourceHandleDecUse method. This
 * releases the handle so it is available to other threads.
 *
 * ## Free
 *
 * A resource manager is freed, along with its resource, using the
 * #fiftyoneDegreesResourceManagerFree method. This prevents any new handles
 * from being acquired, and frees the resource being managed.
 * If the resource has active handles, then a free method does not block.
 * Instead it prevents new handles from being acquired and sets the manager
 * to free the resource once the last handle has been released.
 *
 *
 * ## Replace
 *
 * A resource can be replaced using the #fiftyoneDegreesResourceReplace method.
 * This swaps the resource being managed so that any new requests for a handle
 * return the new resource. The existing resource is freed once the last active
 * handle to it has been released.
 *
 * ## Usage Example
 *
 * ```
 * typedef struct someResourceType {
 *     fiftyoneDegreesResourceHandle *handle; 
 *     void *data;
 * }
 * someResourceType *resource;
 * fiftyoneDegreesResourceManager manager;
 *
 * // Initialise the resource manager with a resource
 * fiftyoneDegreesResourceManagerInit(
 *     &manager,
 *     resource,
 *     &resource->handle,
 *     Free);
 *
 * // Check that the resource handle was set successfully
 * if (resource->handle != NULL) {
 *
 *     // Get a handle to the resource to ensure it is not freed by any
 *     // other threads
 *     someResourceType *localResource = (someResourceType*)
 *         fiftyoneDegreesResourceHandleIncUse(&manager)->resource;
 *
 *     // Free the resource. This operation will be postponed until there are
 *     // no remaining handled being used
 *     fiftyoneDegreesResourceManagerFree(&manager);
 *
 *     // Do something with the resource while it is guaranteed to be available
 *     // ...
 *
 *     // Release the resource so other threads know it is eligible for freeing
 *     fiftyoneDegreesResourceHandleDecUse(resource->handle);
 *
 *     // This is the point where the call to free the manager will actually
 *     // be carried out now that nothing is referencing the resource
 * }
 * ```
 *
 * @{
 */

/* Define NDEBUG if needed, to ensure asserts are disabled in release builds */
#if !defined(DEBUG) && !defined(_DEBUG) && !defined(NDEBUG)
#define NDEBUG
#endif

#include <stdlib.h>
#include <stdint.h>
#include <assert.h>
#include "threading.h"
#include "common.h"

/** @cond FORWARD_DECLARATIONS */
typedef struct fiftyone_degrees_resource_manager_t
	fiftyoneDegreesResourceManager;

typedef struct fiftyone_degrees_resource_handle_t
    fiftyoneDegreesResourceHandle;
/** @endcond */

/**
 * Handle for a shared resource. The first data structure counter tracks use
 * of the resource and free resources that are not longer active.
 * Counter must be the first member to ensure correct memory aligned for 
 * interlocked operations.
 */
typedef struct fiftyone_degrees_resource_handle_t {
#ifndef FIFTYONE_DEGREES_NO_THREADING
    volatile 
#endif 
    fiftyoneDegreesInterlockDoubleWidth counter; /**< Counter for this 
                                                 handle. */
    const void* resource; /**< Pointer to the resource being managed. */
    const fiftyoneDegreesResourceManager* manager; /**< Pointer to the manager
                                                   the handle relates to. */
    void(*freeResource)(void*); /**< Pointer to the method used to free the
                                resource. */
} fiftyoneDegreesResourceHandle;

/**
 * Manager structure used to provide access to a shared and changing resource.
 */
typedef struct fiftyone_degrees_resource_manager_t {
#ifndef FIFTYONE_DEGREES_NO_THREADING
    volatile fiftyoneDegreesResourceHandle *active; /**< Current handle
                                                    for resource used 
                                                    by the manager. */
#else
	fiftyoneDegreesResourceHandle *active; /**< Non volatile current handle for
										   the resource used by the manager. */
#endif
} fiftyoneDegreesResourceManager;

/**
 * Initialise a preallocated resource manager structure with a resource for it
 * to manage access to.
 * The resourceHandle parameter must point to the handle within the resource
 * under management so that the handle can be assigned to the resource before
 * the resource is placed under management.
 *
 * @param manager the resource manager to initialise with the resource
 * @param resource pointer to the resource which the manager should manage
 * access to
 * @param resourceHandle points to the location the new handle should be stored
 * @param freeResource method to use when freeing the resource
 */
EXTERNAL void fiftyoneDegreesResourceManagerInit(
	fiftyoneDegreesResourceManager *manager,
	void *resource,
	fiftyoneDegreesResourceHandle **resourceHandle,
	void(*freeResource)(void*));

/**
 * Frees any data associated with the manager and releases the manager. All 
 * memory is released after this operation.
 *
 * @param manager the resource manager to be freed
 */
EXTERNAL void fiftyoneDegreesResourceManagerFree(
	fiftyoneDegreesResourceManager *manager);

/**
 * Increments the usage counter for the resource and returns a handle that can
 * be used to reference it. The handle **MUST** be used to decrement the use
 * count using the #fiftyoneDegreesResourceHandleDecUse method when the
 * resource is finished with. The resource can be guaranteed not to be freed
 * until after the decrement method has been called.
 * @param manager the resource manager to initialise with the resource
 */
EXTERNAL fiftyoneDegreesResourceHandle* fiftyoneDegreesResourceHandleIncUse(
	fiftyoneDegreesResourceManager *manager);

/**
 * Decrements the usage counter. If the count reaches zero then resource will
 * become eligible to be freed either when the manager replaces it or when the
 * manager is freed.
 * @param handle to the resource which should be released by the manager
 */
EXTERNAL void fiftyoneDegreesResourceHandleDecUse(
	fiftyoneDegreesResourceHandle *handle);

/**
 * Return the current usage counter.
 * WARNING: This call is not thread-safe and is suitable for using in
 * testing only.
 * @param handle to the resource which should be released by the manager
 * @return the current usage counter
 */
EXTERNAL int32_t fiftyoneDegreesResourceHandleGetUse(
	fiftyoneDegreesResourceHandle *handle);

/**
 * Replaces the resource with the new resource. If the existing resource is 
 * not being used it will be freed. Otherwise it is left to the decrement 
 * function to free the resource when the usage count is zero.
 * @param manager the resource manager to initialise with the resource
 * @param newResource pointer to the resource which the manager should manage
 * access to
 * @param newResourceHandle points to the location the new handle should be
 * stored
 */
EXTERNAL void fiftyoneDegreesResourceReplace(
	fiftyoneDegreesResourceManager *manager,
	void *newResource,
	fiftyoneDegreesResourceHandle **newResourceHandle);

/**
 * @}
 */

#endif
