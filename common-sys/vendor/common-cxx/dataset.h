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

#ifndef FIFTYONE_DEGREES_DATASET_H_INCLUDED
#define FIFTYONE_DEGREES_DATASET_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesDataSet Data Set
 *
 * A data file initialised in a structure.
 *
 * ## Terms
 *
 * **Get** : returns an immutable data set that must be released when the
 * caller is finished with it.

 * **Release** : releases a reference on the data set returned from the Get
 * operation.

 * **Reload** : reloads an existing data set while maintaining any current
 * references to the existing data set.
 *
 * ## Introduction
 *
 * A DataSet is a data file initialised in a structure that can be used to
 * process data. Collections from the file may be stored in memory or streamed
 * from the file when needed.
 *
 * A DataSet is used to process data, in most cases this is in the form of
 * evidence. Values for the Properties which the data set is capable of
 * returning can then be retrieved from the result of processing.
 *
 * ## Creation
 *
 * A DataSet is created by allocating the structure and initialising from one
 * of the following:
 *
 * **File** : a data file is either read into memory or handle maintained for
 * streaming by the data set.
 *
 * **Memory** : a data file read into continuous memory is used by the data set.
 *
 * ## Operation
 *
 * A DataSet is a resource to be maintained by a Resource Manager. So any
 * thread wanting to use it must get a reference from the manager
 * (see resource.h).
 *
 * The data set implementation extending will contain methods to process data.
 * Usually these will return a Results instance (or an extending structure),
 * see results.h for more details.
 *
 * ## Reloading
 *
 * A DataSet can be reloaded without interrupting operation by using the 
 * defined Reload methods. These take either a new data file or a new memory
 * pointer, initialise a new data set, and replace the existing one in a
 * thread-safe manor.
 *
 * ## Free
 *
 * A DataSet is a managed resource, so it should not be freed directly. Instead
 * the manager should be freed, so that the data set is safely freed without
 * impacting other threads.
 *
 * @{
 */

#include "exceptions.h"
#include "file.h"
#include "memory.h"
#include "resource.h"
#include "properties.h"
#include "headers.h"
#include "config.h"
#include "overrides.h"
#include "common.h"
#include "indices.h"

/**
 * Base data set structure which contains the 'must have's for all data sets.
 */
typedef struct fiftyone_degrees_dataset_base_t {
	const char fileName[FIFTYONE_DEGREES_FILE_MAX_PATH]; /**< Path to the file
	                                                     used to initialise the
	                                                     data set. This will
	                                                     either be the same as
	                                                     the master file name,
	                                                     or the path to the
	                                                     temp file being used. */
	const char masterFileName[FIFTYONE_DEGREES_FILE_MAX_PATH]; /**< Path to the
	                                                           file used to
	                                                           initialise the
	                                                           data set. */
	void *memoryToFree; /**< A pointer to the memory where the data within the
						data set is held. NULL if the entire data set is not
						held in memory, the caller is going to free the
						memory or there are a mixture of caches and
						partially allocated memory in use. */
	bool isInMemory; /**< True if the data file has been read into continuous
						 memory */
	fiftyoneDegreesFilePool filePool; /**< Linked list of file readers */
	fiftyoneDegreesResourceHandle *handle; /**< Pointer to the handle that
										   tracks this data set instance */
	fiftyoneDegreesPropertiesAvailable *available; /**< Available properties */
	fiftyoneDegreesHeaders *uniqueHeaders; /**< Related HTTP headers */
	fiftyoneDegreesOverridePropertyArray *overridable; /**< Array of properties
													   that can be 
													   overridden */
	fiftyoneDegreesIndicesPropertyProfile* indexPropertyProfile; /**< Index to 
															   look up profile 
															   values by 
															   property */
    const void *config; /**< Pointer to the config used to create the dataset */
} fiftyoneDegreesDataSetBase;

/**
 * Initialses the data set from data stored in continuous memory.
 * @param dataSet pointer to the data set to be initialised
 * @param config configuration for the operation of the data set, or NULL if 
 * default configuration is required
 * @param properties the properties that will be consumed from the data set, or 
 * NULL if all available properties in the hash data file should be available
 * for consumption
 * @param memory pointer to continuous memory containing the data set
 * @param size the number of bytes that make up the data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set intialisation. Any value
 * other than #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not
 * initialised correctly
 */
typedef fiftyoneDegreesStatusCode(*fiftyoneDegreesDataSetInitFromMemoryMethod)(
	void *dataSet,
	const void *config,
	fiftyoneDegreesPropertiesRequired *properties,
	void *memory,
	fiftyoneDegreesFileOffset size,
	fiftyoneDegreesException *exception);

/**
 * Initialses the data set from data stored on file. This method
 * should clean up the resource properly if the initialisation process fails.
 * That means all allocated memory should be freed and pointers to these
 * memorys should be set to NULL. The input dataSet should also be freed.
 * @param dataSet pointer to the data set to be initialised
 * @param config configuration for the operation of the data set, or NULL if
 * default configuration is required
 * @param properties the properties that will be consumed from the data set, or
 * NULL if all available properties in the hash data file should be available
 * for consumption
 * @param fileName the full path to a file with read permission that contains 
 * the data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the status associated with the data set intialisation. Any value
 * other than #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not
 * initialised correctly
 */
typedef fiftyoneDegreesStatusCode(*fiftyoneDegreesDataSetInitFromFileMethod)(
	void *dataSet,
	const void *config,
	fiftyoneDegreesPropertiesRequired *properties,
	const char *fileName,
	fiftyoneDegreesException *exception);

/**
 * Initialises the properties in the data set. Usually this means constructing
 * an array of pointers to the properties which are required for quick access.
 * @param dataSet pointer to a valid data set
 * @param properties the properties which should be initialised in the data set
 * @param state pointer to data which is needed by getPropertymethod
 * @param getPropertyMethod method used to retrieve the name of a property at
 * a specified index from the data set
 * @param getEvidencePropertiesMethod method used to populate the list of
 * evidence required for a property in the data set
 * @return the status associated with the property initialisation. Any value
 * other than #FIFTYONE_DEGREES_STATUS_SUCCESS  means the properties were not
 * initialised correctly
 */
fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitProperties(
	fiftyoneDegreesDataSetBase *dataSet,
	fiftyoneDegreesPropertiesRequired *properties,
	void *state,
	fiftyoneDegreesPropertiesGetMethod getPropertyMethod,
    fiftyoneDegreesEvidencePropertiesGetMethod getEvidencePropertiesMethod);

/**
 * Initialises the HTTP headers in the data set. Usually this means
 * constructing an array of pointers to the headers which are required for
 * quick access.
 * @param dataSet pointer to a valid data set
 * @param state pointer to data which is needed by getPropertymethod
 * @param getHeaderMethod method used to retrieve the unique id and name of a
 * header at a specified index from the data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the header initialisation. Any value
 * other than #FIFTYONE_DEGREES_STATUS_SUCCESS  means the headers were not
 * initialised correctly
 */
fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitHeaders(
	fiftyoneDegreesDataSetBase *dataSet,
	void *state,
	fiftyoneDegreesHeadersGetMethod getHeaderMethod,
	fiftyoneDegreesException* exception);

/**
 * Initialses the data set from data stored on file.
 * @param dataSet pointer to the pre allocated data set to be initialised
 * @param fileName the full path to a file with read permission that contains
 * the data set
 * @param bytesToCompare the number of bytes to compare if the reuse temporary
 * file option is enabled. If a temporary file is found and the first bytes are
 * equal to the master file, then the file is used, if not then a new temporary
 * file is created
 * @return the status associated with the data set intialisation. Any value
 * other than #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not
 * initialised correctly
 */
fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitFromFile(
	fiftyoneDegreesDataSetBase *dataSet,
	const char *fileName,
	fiftyoneDegreesFileOffset bytesToCompare);

/**
 * Initialses the data set from data stored in continuous memory.
 * @param dataSet pointer to the pre allocated data set to be initialised
 * @param reader constructed to read the memory containing the data set
 * @return the status associated with the data set intialisation. Any value
 * other than #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not
 * initialised correctly
 */
fiftyoneDegreesStatusCode fiftyoneDegreesDataSetInitInMemory(
	fiftyoneDegreesDataSetBase *dataSet,
	fiftyoneDegreesMemoryReader *reader);

/**
 * Resets a newly allocated data set structure ready for initialisation.
 * @param dataSet pointer to the allocated data set
 */
void fiftyoneDegreesDataSetReset(fiftyoneDegreesDataSetBase *dataSet);

/**
 * Gets a pointer to the active data set from a resource manager.
 * Note that when this is finished with it must be released with the
 * corresponding release method.
 * @param manager pointer to the manager which manages the data set resource
 * @return pointer to the data set resource
 */
EXTERNAL fiftyoneDegreesDataSetBase* fiftyoneDegreesDataSetGet(
	fiftyoneDegreesResourceManager *manager);

/** 
 * Releases a reference to a data set which has been fetched via the DataSetGet
 * method.
 * @param dataSet pointer to the data set to release
 */
EXTERNAL void fiftyoneDegreesDataSetRelease(fiftyoneDegreesDataSetBase *dataSet);

/**
 * Closes the data set by freeing anything which has been initialised at
 * creation. This does not free the data set structure itself.
 * @param dataSet pointer to the data set to complete
 */
void fiftyoneDegreesDataSetFree(fiftyoneDegreesDataSetBase *dataSet);

/**
 * Reload the data set being used by the resource manager using a data file
 * loaded into contiguous memory. When initialising the data, the configuration
 * that manager was first created with is used.
 *
 * If the data passed in is successfully initialised, the current data set is
 * replaced The old data will remain in memory until the last reference to it
 * is released.
 * @param manager pointer to the resource manager to reload the data set for
 * @param source pointer to the memory location where the new data file is
 * stored
 * @param length of the data in memory
 * @param dataSetSize size of the data set structure to allocate for the new
 * data set
 * @param initDataSet init method used to initialise the new data set from the
 * memory pointer provided
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set reload. Any value other than
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded
 * correctly
 */
fiftyoneDegreesStatusCode fiftyoneDegreesDataSetReloadManagerFromMemory(
	fiftyoneDegreesResourceManager *manager,
	void *source,
	fiftyoneDegreesFileOffset length,
	size_t dataSetSize,
	fiftyoneDegreesDataSetInitFromMemoryMethod initDataSet,
	fiftyoneDegreesException *exception);

/**
 * Reload the data set being used by the resource manager using the data file
 * location specified. When initialising the data, the configuration that
 * manager was first created with is used.
 *
 * If the new data file is successfully initialised, the current data set is
 * replaced The old data will remain in memory until the last reference to it
 * is released.
 * @param manager pointer to the resource manager to reload the data set for
 * @param fileName path to the new data file
 * @param dataSetSize size of the data set structure to allocate for the new
 * data set
 * @param initDataSet init method used to initialise the new data set from the
 * file provided
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set reload. Any value other than
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded
 * correctly
 */
fiftyoneDegreesStatusCode fiftyoneDegreesDataSetReloadManagerFromFile(
	fiftyoneDegreesResourceManager* manager,
	const char *fileName,
	size_t dataSetSize,
	fiftyoneDegreesDataSetInitFromFileMethod initDataSet,
	fiftyoneDegreesException *exception);

/**
 * Reload functions are common across all data set implementations where
 * the naming of the data set type and the init methods comform to the common
 * pattern. This macro requires the init methods to be static and of the
 * form initDataFrom[Memory|File]. The data set name must be DataSet[Type].
 * @param t the name of the resource type to define reload methods for
 */
#define FIFTYONE_DEGREES_DATASET_RELOAD(t) \
/** \
 * Reload the data set being used by the resource manager using a data file \
 * loaded into contiguous memory. When initialising the data, the configuration \
 * that manager was first created with is used. \
 * \
 * If the data passed in is successfully initialised, the current data set is \
 * replaced The old data will remain in memory until the last reference to it \
 * is released. \
 * @param manager pointer to the resource manager to reload the data set for \
 * @param source pointer to the memory location where the new data file is \
 * stored \
 * @param length of the data in memory \
 * @param exception pointer to an exception data structure to be used if an \
 * exception occurs. See exceptions.h \
 * @return the status associated with the data set reload. Any value other than \
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded \
 * correctly \
 */ \
fiftyoneDegreesStatusCode fiftyoneDegrees##t##ReloadManagerFromMemory( \
fiftyoneDegreesResourceManager *manager, \
void *source, \
fiftyoneDegreesFileOffset length, \
fiftyoneDegreesException *exception) { \
	return fiftyoneDegreesDataSetReloadManagerFromMemory( \
		manager, \
		source, \
		length, \
		sizeof(DataSet##t), \
		initDataSetFromMemory, \
		exception); \
} \
/** \
 * Reload the data set being used by the resource manager using the data file \
 * location specified. When initialising the data, the configuration that \
 * manager was first created with is used. \
 * \
 * If the new data file is successfully initialised, the current data set is \
 * replaced The old data will remain in memory until the last reference to it \
 * is released. \
 * @param manager pointer to the resource manager to reload the data set for \
 * @param fileName path to the new data file \
 * @param exception pointer to an exception data structure to be used if an \
 * exception occurs. See exceptions.h \
 * @return the status associated with the data set reload. Any value other than \
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded \
 * correctly \
 */ \
fiftyoneDegreesStatusCode fiftyoneDegrees##t##ReloadManagerFromFile( \
fiftyoneDegreesResourceManager* manager, \
const char *fileName, \
fiftyoneDegreesException *exception) { \
	return fiftyoneDegreesDataSetReloadManagerFromFile( \
		manager, \
		fileName, \
		sizeof(DataSet##t), \
		initDataSetFromFile, \
		exception); \
} \
/** \
 * Reload the data set being used by the resource manager using the data file \
 * which the data set was initialised with. When initialising the data, the
 * configuration that manager was first created with is used. \
 * \
 * If the new data file is successfully initialised, the current data set is \
 * replaced The old data will remain in memory until the last reference to it \
 * is released. \
 * @param manager pointer to the resource manager to reload the data set for \
 * @param exception pointer to an exception data structure to be used if an \
 * exception occurs. See exceptions.h \
 * @return the status associated with the data set reload. Any value other than \
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded \
 * correctly \
  */ \
fiftyoneDegreesStatusCode fiftyoneDegrees##t##ReloadManagerFromOriginalFile( \
fiftyoneDegreesResourceManager* manager, \
fiftyoneDegreesException *exception) { \
	fiftyoneDegreesStatusCode status = FIFTYONE_DEGREES_STATUS_NOT_SET; \
	DataSet##t* dataSet = fiftyoneDegreesDataSet##t##Get(manager); \
	status = fiftyoneDegrees##t##ReloadManagerFromFile( \
		manager, \
		dataSet->b.b.masterFileName, \
		exception); \
	fiftyoneDegreesDataSet##t##Release(dataSet); \
	return status; \
}

/**
 * @}
 */

#endif
