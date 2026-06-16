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

#ifndef FIFTYONE_DEGREES_COLLECTION_H_INCLUDED
#define FIFTYONE_DEGREES_COLLECTION_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesCollection Collection
 *
 * Group of related items such as strings.
 *
 * ## Terms
 *
 * **Collection** : a group of items that share a common data structure.
 *
 * **Item** : an immutable entity within the collection. The number of bytes
 * used to store the item may be either fixed or variable.
 *
 * **Get** : returns an immutable Item that must be released when the caller is
 * finished with it.
 *
 * **Release** : releases a lock on the Item returned from the Get operation.
 *
 * **Index** : where the Item data uses a fixed number of bytes the index of
 * the item in the Collection.
 *
 * **Offset** : where items do not share the same number of data bytes the 0
 * based offset to the Item from the start of the collection’s data (after any
 * header).
 * 
 * ## Introduction
 * 
 * A Collection is a group of related Items such as strings or properties where 
 * the consumer wishes to access the items without concern for the underlying 
 * storage mechanism and memory management. Immutable items can be retrieved 
 * (Get) from the collection which **MUST** be released once the caller has
 * finished with the data.
 * 
 * In some configurations it may be more efficient to load all related Items
 * into memory in a continuous address space exactly matching the data structure
 * used in the source data. In others it may be desirable to only load in memory
 * the Items from the source data when required. In all configurations the 
 * logical methods to retrieve and release these items are identical.
 * 
 * The methods in the Collection abstract the implementation for collection of 
 * items to support the following configurations:
 * 
 * **Memory** : all the Items in the Collection are held in a continuous
 * address  space. The fastest access option where all memory can be allocated
 * during initialisation.
 * 
 * **File** : none of the Items are loaded into memory and they are retrieved
 * from the data source when required. The slowest access option but the most
 * memory efficient. The memory needed to store the item is allocated when the
 * Item is requested and deallocated when released.
 * 
 * **Cache** : a hybrid mode where the Least Recently Used (LRU) cache is used
 * to store a limited number of Items in memory. Cache is a compromise between
 * in memory and file based configurations. Unlike Memory and File the cache
 * needs to be locked when accessed for both Get and Release and performance
 * may degrade when used in a multi threaded configuration.
 * 
 * Sometimes it may be desirable to use multiple configurations of Collection
 * with the same underlying data. Consider a data structure where the most 
 * frequently required Items exist at the start of the structure. These Items
 * would benefit from being held in memory for fast retrieval whilst the
 * remaining Items which may be infrequently required are cached or loaded when 
 * needed from the file. For these situations Collections can be chained
 * together such that the first Collection may not contain data for the
 * requested index or offset and can pass the retrieval to the next Collection
 * in the chain. It is important to note that all chained Collections must be a
 * subset of the final Collection in the chain (as opposed to additional 
 * Collections), i.e. all elements are 'gettable' from the final Collection.
 * 
 * Once a Collection is created the only methods that should be used to access 
 * data are those exposed in the fiftyoneDegreesCollection instance. It is for 
 * this reason the collection.h file exposes so few external methods.
 * 
 * ## Creation
 * 
 * There are two Collection creation methods:

 * ### #fiftyoneDegreesCollectionCreateFromMemory  
 * 
 * Creates a Collection that is created to access memory that is already
 * allocated in continuous memory simply maps to this memory and provides 
 * relatively simple methods to retrieve Items. No reference to the underlying 
 * data file or source is required but the memory must already be allocated.
 * 
 * ### #fiftyoneDegreesCollectionCreateFromFile 
 * 
 * A Collection mapped to a data file is more complex than the memory method
 * and a greater set of creation configuration features are required. For
 * example; caching options or how many Items at the start of the Collection to
 * store in memory.
 * 
 * A #fiftyoneDegreesCollectionConfig structure is used to configure the 
 * operation of the resulting Collection. This structure has the following 
 * fields.
 * 
 * **loaded** : the number of Items to load into memory from the start of the
 * Collection’s data structure.
 *
 * **capacity** : 0 if a cache should not be used, otherwise the capacity of
 * cache.

 * **concurrency** : the expected number of concurrent operations, 1 or greater.
 * 
 * The file create method will work out the different types of Collection(s)
 * needed and how to chain them based on the configuration provided.
 * 
 * Additionally, a pointer to a method to read an Item into the Collection from 
 * the underlying data file is required. For fixed width Collections the 
 * fiftyoneDegreesCollectionReadFileFixed can be used where the Item key is the
 * zero based index in the Collection. For variable byte width Collections the
 * consumer will need to provide a method that implements
 * #fiftyoneDegreesCollectionFileRead. 
 * 
 * All creation methods return a pointer to an instance of 
 * #fiftyoneDegreesCollection. Following creation only the methods exposed by 
 * this Collection should be used to retrieve and release Items.
 * 
 * ## Free
 * 
 * The freeCollection method of the Collection must be called when the
 * Collection is finished with. After this method is called the Collection data
 * structure and all owned data will be released.
 * 
 * ## Get & Release
 * 
 * All operations that retrieve data from the Collection require an instance of
 * #fiftyoneDegreesCollectionItem to be passed. Items can be reused across
 * multiple Get and Release operations to reduce the frequency of memory
 * allocations. Items contain a handle to the underlying data which might be
 * used during the Release operation.
 *
 * ## Usage Example
 *
 * ```
 * FILE *file;
 * fiftyoneDegreesFilePool *filePool;
 * FIFTYONE_DEGREES_EXCEPTION_CREATE;
 * const fiftyoneDegreesCollectionConfig *config;
 * fiftyoneDegreesCollectionItem item;
 *
 * // Read the collection header from file which contains 32 bit integers
 * fiftyoneDegreesCollectionHeader *header = fiftyoneDegreesCollectionHeaderFromFile(
 *     file,
 *     sizeof(int32_t),
 *     true);
 *
 * // Create the collection from a file which contains 32 bit integers
 * fiftyoneDegreesCollection *collection = fiftyoneDegreesCollectionCreateFromFile(
 *     file,
 *     filePool,
 *     config,
 *     header,
 *     fiftyoneDegreesCollectionGetInteger32);
 *
 * // Check the collection was create successfully
 * if (collection != NULL) {
 *
 *     // Initialise the item ready to store data from the collection
 *     fiftyoneDegreesDataReset(&item.data);
 *
 *     // Get a pointer to the value from the collection
 *     int32_t *valuePtr = collection->get(
 *         collection,
 *         0,
 *         &item,
 *         exception);
 *
 *     // Check the value was returned
 *     if (valuePtr != NULL) {
 *
 *         // Get the value from the pointer
 *         int32_t value = *valuePtr;
 *
 *         // Release the value back to the collection
 *         FIFTYONE_DEGREES_COLLECTION_RELEASE(collection, &item);
 *     }
 *
 *     // Free the collection
 *     FIFTYONE_DEGREES_COLLECTION_FREE(collection);
 * }
 * ```
 *
 * ## Important Considerations
 * 
 * The Item passed to the Get method **MUST** have had
 * #fiftyoneDegreesDataReset called on the data field before use to set the
 * default field values. Without this reset the Collection will not know if it
 * can reuse any memory already allocated from a previous Get operation that is
 * no longer being referenced. For example, if a large string was loaded into
 * an Item, and the Item is then reused for a shorter string, there is no need
 * to reallocate memory. Therefore, the same memory is reused.
 * 
 * The get and release methods of the collection are the only methods that 
 * should be used to retrieve items from a collection. An item retrieved with 
 * the get method **MUST** be released when finished with via a call to the 
 * collection’s release method.
 * 
 * A call to Get **DOES NOT** need to set the Collection field of the Item.
 * Collection is not a mandatory field to avoid the overhead of setting the
 * Collection field where the Collection’s implementation of the Release method
 * does nothing. This is important to avoid a performance penalty when all
 * Collection data is in memory and does not need to be released. For this
 * reason the consumer must always reference the Collection directly and not
 * via the Collection field of the Item. The Collection field is used by 
 * some internal 51Degrees method where a specific Get method is used that will
 * always set the Collection.
 *
 * @{
 */

 /* Define NDEBUG if needed, to ensure asserts are disabled in release builds */
#if !defined(DEBUG) && !defined(_DEBUG) && !defined(NDEBUG)
#define NDEBUG
#endif

#include "collectionKey.h"
#include "data.h"
#include "exceptions.h"
#include "cache.h"
#include "file.h"
#include "memory.h"
#include "common.h"
#include <stdio.h>
#include <string.h>

#include "propertyValueType.h"

/**
 * Free a collection by checking if it is NULL first.
 * @param c collection to free
 */
#define FIFTYONE_DEGREES_COLLECTION_FREE(c) \
if (c != NULL) { c->freeCollection(c); }

/**
 * Collection release macro used to release a collection item. This should
 * always be used in place of the release method to enable methods to be
 * optimised out at compile time when memory only mode is set.
 * @param c collection the item belongs to
 * @param i item to release
 */
#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
#define FIFTYONE_DEGREES_COLLECTION_RELEASE(c, i) { assert(!((i)->collection) || (c == (i)->collection)); c->release(i); };
#else
#define FIFTYONE_DEGREES_COLLECTION_RELEASE(c, i)
#endif

/**
 * Collection header structure which defines the size and location of the
 * collection data.
 */
#pragma pack(push, 1)
typedef struct fiftyone_degrees_collection_header_t {
	fiftyoneDegreesFileOffsetUnsigned startPosition; /**< Start position in the data file of the entities */
	uint32_t length; /**< Length in bytes of all the entities */
	uint32_t count; /**< Number of entities in the collection */
} fiftyoneDegreesCollectionHeader;
#pragma pack(pop)

/**
 * Collection configuration structure which defines how the collection should
 * be created by the create methods.
 */
typedef struct fiftyone_degrees_collection_config_t {
	bool loaded; /**< Collection is loaded entirely into memory */
	uint32_t capacity; /**< Number of items the cache should store, 0 for no
	                       cache */
	uint16_t concurrency; /**< Expected number of concurrent requests, 1 or
						      greater */
} fiftyoneDegreesCollectionConfig;

/** @cond FORWARD_DECLARATIONS */
typedef struct fiftyone_degrees_collection_t fiftyoneDegreesCollection;
typedef struct fiftyone_degrees_collection_item_t fiftyoneDegreesCollectionItem;
typedef struct fiftyone_degrees_collection_file_t fiftyoneDegreesCollectionFile;
/** @endcond */

/**
 * Used to store a handle to the underlying item that could be used to release
 * the item when it's finished with. Also contains a reference to the data 
 * structure used to handle memory allocation enabling memory to be reused
 * across multiple gets reducing the frequency of memory operations.
 */
typedef struct fiftyone_degrees_collection_item_t {
	fiftyoneDegreesData data; /**< Item data including allocated memory */
	void *handle; /**< A handle that relates to the data. i.e. a cache node */

	/**
	 * Collection the item came from which may not have been set.
	 * Should not be used by external code.
	 */
	const fiftyoneDegreesCollection *collection;
} fiftyoneDegreesCollectionItem;

/**
 * Gets an item from the collection. Returns a pointer to the item, or NULL
 * if the item could not be loaded. The exception parameter is set to the 
 * status code to indicate the failure.
 * @param collection pointer to the file collection
 * @param key key of the item in the data structure
 * @param item pointer to the item structure to place the result in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the value in the data->ptr field, or NULL if not successful
 */
typedef void* (*fiftyoneDegreesCollectionGetMethod)(
	const fiftyoneDegreesCollection *collection,
	const fiftyoneDegreesCollectionKey *key,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Reads the item from the underlying data file. Used by the file related
 * collection methods.
 * @param collection pointer to the file collection
 * @param key key of the item in the data structure
 * @param data pointer to the data structure to store the item
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the value in the data->ptr field, or NULL if not successful
 */
typedef void* (*fiftyoneDegreesCollectionFileRead)(
	const fiftyoneDegreesCollectionFile *collection,
	const fiftyoneDegreesCollectionKey *key,
	fiftyoneDegreesData *data,
	fiftyoneDegreesException *exception);

/**
 * Compares two items and returns the difference between them for the purposes
 * of a binary search of ordering operation.
 * @param state to be used for the comparison
 * @param item the value to compare against the state
 * @param key key of the item in the data structure
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return negative if a is lower than b, positive if a is higher than b or 0 
 * if equal.
 */
typedef int(*fiftyoneDegreesCollectionItemComparer)(
	void *state,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesCollectionKey key,
	fiftyoneDegreesException *exception);

/**
 * Releases the item so that the collection can free the memory or take
 * other actions when the caller no longer needs access to the item.
 * @param item the item returned from Get to be released
 */
typedef void (*fiftyoneDegreesCollectionReleaseMethod)(
	fiftyoneDegreesCollectionItem* item);

/**
 * Frees all the memory and handles associated with the collection.
 * @param collection the collection to be freed
 */
typedef void(*fiftyoneDegreesCollectionFreeMethod)(
	fiftyoneDegreesCollection* collection);

/**
 * Method used to iterate over data held in a collection.
 * @param state state data for all callbacks
 * @param key the key used with get method of the item being returned
 * @param data data for the specific item
 * @return true if the iteration should continue, otherwise false to stop it
 */
typedef bool(*fiftyoneDegreesCollectionIterateMethod)(
	void *state,
	uint32_t key,
	void *data);

/**
 * All the shared methods and fields required by file, memory and cached
 * collections. The state field points to the specific collection data 
 * structure and handles.
 */
typedef struct fiftyone_degrees_collection_t {
	fiftyoneDegreesCollectionGetMethod get; /**< Gets an entry into the 
	                                        item provided. The consumer MUST 
	                                        release the item when it has 
	                                        finished with the data */
	fiftyoneDegreesCollectionReleaseMethod release; /**< Releases the item 
													handle */
	fiftyoneDegreesCollectionFreeMethod freeCollection; /**< Frees collection
	                                                    and all items retrieved 
	                                                    from the collection 
	                                                    will become invalid */
	void *state; /**< Pointer to data for memory, cache or file. Either a 
	                #fiftyoneDegreesCollectionMemory,
	                #fiftyoneDegreesCollectionFile or 
	                #fiftyoneDegreesCollectionCache */
	uint32_t count; /**< The number of items, or 0 if not available */
	uint32_t elementSize; /**< The size of each entry, or 0 if variable length */
	uint32_t size; /**< Number of bytes in the source data structure containing
					  the collection's data */
	const char *typeName; /**< Name of collection type (vtable). */
} fiftyoneDegreesCollection;

/**
 * Type of collection where the collection is held in continuous memory.
 */
typedef struct fiftyone_degrees_collection_memory_t {
	fiftyoneDegreesCollection *collection; /**< The generic collection */
	byte *firstByte; /**< The first byte in memory of the collection */
	byte *lastByte; /**< The last byte in memory of the collection */
	void *memoryToFree; /**< Memory to free when freeing the collection, or NULL
						if no memory to free*/
} fiftyoneDegreesCollectionMemory;

/**
 * Type of collection where the collection is streamed from file.
 */
typedef struct fiftyone_degrees_collection_file_t {
	fiftyoneDegreesCollection *collection; /**< The generic collection */
	fiftyoneDegreesFilePool *reader; /**< Reader used to load items into the 
									 cache, or NULL if no cache */
	fiftyoneDegreesFileOffset offset; /**< Offset to the collection in the source data structure */
	fiftyoneDegreesCollectionFileRead read; /**< Read method used to read an
											item from file at an offset or
											index */
} fiftyoneDegreesCollectionFile;

/**
 * Type of collection where items are cached as they are used. This is usually
 * used in conjunction with a file cache to speed up operation while
 * maintaining a small memory footprint.
 */
typedef struct fiftyone_degrees_collection_cache_t {
	fiftyoneDegreesCollection *source; /**< The source collection used to load
									   items into the cache */
	fiftyoneDegreesCache *cache; /**< Loading cache to use as data source */
} fiftyoneDegreesCollectionCache;

/**
 * Determines if in memory collection methods have been compiled so they are
 * fully optimized. This results in the loss of file stream operation.
 * In memory only operation compiling without stream capabilities using the
 * `FIFTYONE_DEGREES_MEMORY_ONLY` directive results in performance improvements.
 * @return true if the library is compiled for memory only operation, otherwise
 * false.
 */
EXTERNAL bool fiftyoneDegreesCollectionGetIsMemoryOnly();

/**
 * Returns a 32 bit integer from collections that provide such values.
 * @param collection the collection of 32 bit integers
 * @param indexOrOffset the index or offset of the integer required
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the 32 bit integer at the index or offset provided
 */
EXTERNAL int32_t fiftyoneDegreesCollectionGetInteger32(
	const fiftyoneDegreesCollection *collection,
	uint32_t indexOrOffset,
	fiftyoneDegreesException *exception);

/**
 * Creates a collection from the file handle at the current position in the 
 * file. The first 4 bytes read will be the number if fixed with items in the 
 * collection OR the number of bytes until the end of the collection data.
 * @param file a file handle positioned at the start of the collection
 * @param reader a pool of file handles to use operationally to retrieve data
 * from the file after the collection has been created. These are often shared
 * across different collections accessing the same data file.
 * @param config settings for the implementation of the collection to be used.
 * If FIFTYONE_DEGREES_MEMORY_ONLY is defined, then this is either NULL or it
 * is ignored
 * @param header containing collection structure
 * @param read a pointer to a function to read an item into the collection
 * @return pointer to the new collection, or NULL if something went wrong
 */
EXTERNAL fiftyoneDegreesCollection* fiftyoneDegreesCollectionCreateFromFile(
	FILE *file,
	fiftyoneDegreesFilePool *reader,
	const fiftyoneDegreesCollectionConfig *config,
	fiftyoneDegreesCollectionHeader header,
	fiftyoneDegreesCollectionFileRead read);

/**
 * Creates the collection from a memory reader where the collection maps to
 * the memory allocated to the reader. The resulting collection does not
 * free the memory used to store the data. This method is used where the entire
 * data structure is loaded into continuous memory and provides a high 
 * performance collection in all threading situations.
 * @param reader with access to the allocated memory
 * @param header containing collection structure
 * @return pointer to the memory collection, or NULL if the collection could 
 * not be created
 */
EXTERNAL fiftyoneDegreesCollection* fiftyoneDegreesCollectionCreateFromMemory(
	fiftyoneDegreesMemoryReader *reader,
	fiftyoneDegreesCollectionHeader header);

/**
 * Get a handle from the file pool associated with the collection and position
 * the file handle at the offset provided.
 * @param file pointer to the #fiftyoneDegreesCollectionFile to use for the read
 * @param offset from the start of the data structure, not the entire file, 
 * where the item should be read from
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a file handle for further read operations, or NULL if the offset is
 * invalid, or a handle can not be obtained.
 */
EXTERNAL fiftyoneDegreesFileHandle* fiftyoneDegreesCollectionReadFilePosition(
	const fiftyoneDegreesCollectionFile *file,
	uint32_t offset,
	fiftyoneDegreesException *exception);

/**
 * Used with collections where each item is a fixed number of bytes recorded in
 * elementSize. The method will read that number of bytes into the data item
 * ensuring sufficient memory is allocated.
 * Contained in the collection to avoid repeating this common method across
 * different collection consumers.
 * @param file pointer to the #fiftyoneDegreesCollectionFile to use for the
 * read
 * @param data structure to populate with a reference to the item
 * @param key zero based index of the item required in the fixed with data
 * structure
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to the item in the data structure or NULL if can't be 
 * found due to an invalid index
 */
EXTERNAL void* fiftyoneDegreesCollectionReadFileFixed(
	const fiftyoneDegreesCollectionFile *file,
	const fiftyoneDegreesCollectionKey *key,
	fiftyoneDegreesData *data,
	fiftyoneDegreesException *exception);

/**
 * Reads the 4 bytes at the current reader position and configures the
 * collection header. The 4 bytes can either represent the number of fixed
 * width items in the collection OR the number of bytes that follow the 4 bytes
 * which form the collection. The caller must know the type of structure
 * expected and set the elementSize and isCount parameters.
 * @param file a file handle positioned at the start of the collection
 * @param elementSize if known the size in bytes of each item, or 0 for
 * variable width items
 * @param isCount the number of items in the collection if known
 * @return a header set with the details for the collection
 */
EXTERNAL fiftyoneDegreesCollectionHeader
fiftyoneDegreesCollectionHeaderFromFile(
	FILE *file,
	uint32_t elementSize,
	bool isCount);

/**
 * Reads a variable size item from the file where the initial bytes can be used
 * to calculate the size of the entire item. 
 * @param file pointer to the #fiftyoneDegreesCollectionFile to use for the
 * read
 * @param data structure to populate with a reference to the item
 * @param key key of the item in the data structure
 * @param initial pointer to enough memory to store the initial data
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to the item in the data structure or NULL if can't be
 * found due to an invalid index
 */
EXTERNAL void* fiftyoneDegreesCollectionReadFileVariable(
	const fiftyoneDegreesCollectionFile *file,
	fiftyoneDegreesData *data,
	const fiftyoneDegreesCollectionKey *key,
	void *initial,
	fiftyoneDegreesException *exception);

/**
 * Reads the 4 bytes at the current reader position and configures the 
 * collection header. The 4 bytes can either represent the number of fixed
 * width items in the collection OR the number of bytes that follow the 4 bytes
 * which form the collection. The caller must know the type of structure 
 * expected and set the elementSize and isCount parameters.
 * @param reader with access to the allocated memory
 * @param elementSize if known the size in bytes of each item, or 0 for
 * variable width items
 * @param isCount the number of items in the collection if known
 * @return a header set with the details for the collection
 */
EXTERNAL fiftyoneDegreesCollectionHeader
fiftyoneDegreesCollectionHeaderFromMemory(
	fiftyoneDegreesMemoryReader *reader,
	uint32_t elementSize,
	bool isCount);


/**
 * Where a collection is fixed width and contains an ordered list of items
 * this method is used to perform a divide and conquer search. The state 
 * and the comparer are used to compare the current index with the value
 * being sought. If an item is found in the collection the item parameter
 * will contain that item when the method completes. The caller will therefore
 * need to release the item when it's finished with it.
 * @param collection to be searched
 * @param item memory to be used to store the current value being compared. 
 * Will have a lock on the item at the index returned if an item is found.
 * The caller should release the item when finished with it.
 * @param lowerKey to start the search at
 * @param upperKey to end the search at
 * @param keyType type of lower/upper keys
 * @param state used with the compare method when comparing items
 * @param comparer method used to perform the comparison
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the index of the item if found, otherwise -1.
 */
EXTERNAL long fiftyoneDegreesCollectionBinarySearch(
	const fiftyoneDegreesCollection *collection,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesCollectionIndexOrOffset lowerKey,
	fiftyoneDegreesCollectionIndexOrOffset upperKey,
	const fiftyoneDegreesCollectionKeyType *keyType,
	void *state,
	fiftyoneDegreesCollectionItemComparer comparer,
	fiftyoneDegreesException *exception);

/**
 * Gets the actual number of items in the collection by iterating through to
 * the base collection. In cases where there are chained collections which
 * pre-load and/or cache elements, the first collections may not contain the full
 * collection of elements. Therefore the value of collection->count may not be
 * correct. This method follows the collection->next pointers to get to the
 * base collection containing the true count.
 *
 * It is important to note that this gets the count for a collection of fixed
 * size elements, and does not apply to collections of variable sized elements.
 * @param collection to get the count for
 * @return the number of items in the collection
 */
#define fiftyoneDegreesCollectionGetCount(collection) ((collection)->count)

/**
 * @}
 */

#endif
