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

#ifndef FIFTYONE_DEGREES_IPI_INCLUDED
#define FIFTYONE_DEGREES_IPI_INCLUDED

/**
 * @ingroup FiftyOneDegreesIpIntelligence
 * @defgroup FiftyOneDegreesIpIntelligenceApi IpIntelligence
 *
 * All the functions specific to the IP Intelligence API.
 * @{
 */

#if !defined(DEBUG) && !defined(_DEBUG) && !defined(NDEBUG)
#define NDEBUG
#endif

#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <limits.h>
#include <math.h>
#include <time.h>
#include <ctype.h>
#include <assert.h>
#ifdef _MSC_VER
#include <windows.h>
#endif
#include "common-cxx/data.h"
#include "common-cxx/exceptions.h"
#include "common-cxx/threading.h"
#include "common-cxx/file.h"
#include "common-cxx/collection.h"
#include "common-cxx/evidence.h"
#include "common-cxx/list.h"
#include "common-cxx/resource.h"
#include "common-cxx/properties.h"
#include "common-cxx/status.h"
#include "common-cxx/date.h"
#include "common-cxx/pool.h"
#include "common-cxx/component.h"
#include "common-cxx/property.h"
#include "common-cxx/value.h"
#include "common-cxx/profile.h"
#include "common-cxx/overrides.h"
#include "common-cxx/config.h"
#include "common-cxx/dataset.h"
#include "common-cxx/array.h"
#include "common-cxx/results.h"
#include "common-cxx/float.h"
#include "common-cxx/stringBuilder.h"
#include "common-cxx/weightedItem.h"
#include "ip-graph-cxx/graph.h"

/** Default value for the cache concurrency used in the default configuration. */
#ifndef FIFTYONE_DEGREES_CACHE_CONCURRENCY
#ifndef FIFTYONE_DEGREES_NO_THREADING
#define FIFTYONE_DEGREES_CACHE_CONCURRENCY 10
#else
#define FIFTYONE_DEGREES_CACHE_CONCURRENCY 1
#endif
#endif

/**
 * Default value for the string cache size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_STRING_CACHE_SIZE
#define FIFTYONE_DEGREES_STRING_CACHE_SIZE 10000
#endif
/**
 * Default value for the string cache loaded size used in the default
 * collection configuration.
 */
#ifndef FIFTYONE_DEGREES_STRING_LOADED
#define FIFTYONE_DEGREES_STRING_LOADED true
#endif
/**
 * Default value for the graphs cache size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_IP_GRAPHS_CACHE_SIZE
#define FIFTYONE_DEGREES_IP_GRAPHS_CACHE_SIZE 1000
#endif
/**
 * Default value for the graphs cache loaded size used in the default 
 * collection configuration.
 */
#ifndef FIFTYONE_DEGREES_IP_GRAPHS_LOADED
#define FIFTYONE_DEGREES_IP_GRAPHS_LOADED true
#endif
/**
 * Default value for the graphs cache size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_IP_GRAPH_CACHE_SIZE
#define FIFTYONE_DEGREES_IP_GRAPH_CACHE_SIZE 50000
#endif
/**
 * Default value for the graphs cache loaded size used in the default 
 * collection configuration.
 */
#ifndef FIFTYONE_DEGREES_IP_GRAPH_LOADED
#define FIFTYONE_DEGREES_IP_GRAPH_LOADED true
#endif
/**
 * Default value for the profile groups cache size used in the default 
 * collection configuration.
 */
#ifndef FIFTYONE_DEGREES_PROFILE_GROUPS_CACHE_SIZE
#define FIFTYONE_DEGREES_PROFILE_GROUPS_CACHE_SIZE 50000
#endif
/**
 * Default value for the profile groups cache loaded size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_PROFILE_GROUPS_LOADED
#define FIFTYONE_DEGREES_PROFILE_GROUPS_LOADED false
#endif
/**
 * Default value for the profile cache size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_PROFILE_CACHE_SIZE
#define FIFTYONE_DEGREES_PROFILE_CACHE_SIZE 10000
#endif
/**
 * Default value for the profile cache loaded size used in the default
 * collection configuration.
 */
#ifndef FIFTYONE_DEGREES_PROFILE_LOADED
#define FIFTYONE_DEGREES_PROFILE_LOADED false
#endif
/**
 * Default value for the value cache size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_VALUE_CACHE_SIZE
#define FIFTYONE_DEGREES_VALUE_CACHE_SIZE 500
#endif
/**
 * Default value for the value cache loaded size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_VALUE_LOADED
#define FIFTYONE_DEGREES_VALUE_LOADED false
#endif
/**
 * Default value for the property cache size used in the default collection
 * configuration.
 */
#ifndef FIFTYONE_DEGREES_PROPERTY_CACHE_SIZE
#define FIFTYONE_DEGREES_PROPERTY_CACHE_SIZE 0
#endif
/**
 * Default value for the property cache loaded size used in the default
 * collection configuration.
 */
#ifndef FIFTYONE_DEGREES_PROPERTY_LOADED
#define FIFTYONE_DEGREES_PROPERTY_LOADED true
#endif

/**
 * DATA STRUCTURES
 */

/** Dataset header containing information about the dataset. */
#pragma pack(push, 1)
typedef struct fiftyone_degrees_ipi_dataset_header_t {
	const int32_t versionMajor; /**< Major version of the data file loaded */
	const int32_t versionMinor; /**< Minor version of the data file loaded */
	const int32_t versionBuild; /**< Build version of the data file loaded */
	const int32_t versionRevision; /**< Revision version of the data file 
								   loaded */
	const byte tag[16]; /**< Unique data file tag */
	const byte exportTag[16]; /**< Tag identifying the data file export */
	const int32_t copyrightOffset; /**< Offset of the copyright string in the 
								   strings collection */
	const int16_t age; /**< Age of the data set format */
	const int32_t minUserAgentCount; /**< This is a place holder. Not
									 applicable to IP Intelligence. */
	const int32_t nameOffset; /**< Offset of the data file name in the strings 
							  collection */
	const int32_t formatOffset; /**< Offset of the data file format in the 
								strings collection */
	const fiftyoneDegreesDate published; /**< Date when the data file was 
										 published */
	const fiftyoneDegreesDate nextUpdate; /**< Date when the next data file 
										  will be available */
	const fiftyoneDegreesCollectionHeader strings; /**< Size and location of
												   the strings collection */
	const fiftyoneDegreesCollectionHeader components; /**< Size and location of
													  the components 
													  collection */
	const fiftyoneDegreesCollectionHeader maps; /**< Size and location of the
												maps collection */
	const fiftyoneDegreesCollectionHeader properties; /**< Size and location of
													  the properties 
													  collection */
	const fiftyoneDegreesCollectionHeader values; /**< Size and location of the
												  values collection */
	const fiftyoneDegreesCollectionHeader profiles; /**< Size and location of
													the profiles collection */
	const fiftyoneDegreesCollectionHeader graphs; /**< Headers for component
												  graphs */
	const fiftyoneDegreesCollectionHeader profileGroups; /**< Size and
														  location of the
														  profile group offsets
														  collection */
	const fiftyoneDegreesCollectionHeader propertyTypes; /**< Size and location of
													     the propertyTypes
													     collection */
	const fiftyoneDegreesCollectionHeader profileOffsets; /**< Size and
														  location of the
														  profile offsets
														  collection */
} fiftyoneDegreesDataSetIpiHeader;
#pragma pack(pop)

/**
 * IP Intelligence specific configuration structure.
 */
typedef struct fiftyone_degrees_config_ipi_t {
	fiftyoneDegreesConfigBase b; /**< Base configuration */
	fiftyoneDegreesCollectionConfig strings; /**< Strings collection config */
	fiftyoneDegreesCollectionConfig components; /**< Components collection
												config */
	fiftyoneDegreesCollectionConfig maps; /**< Maps collection config */
	fiftyoneDegreesCollectionConfig properties; /**< Properties collection
												config */
	fiftyoneDegreesCollectionConfig values; /**< Values collection config */
	fiftyoneDegreesCollectionConfig profiles; /**< Profiles collection config 
											  */
	fiftyoneDegreesCollectionConfig graphs; /**< Graphs config */
	fiftyoneDegreesCollectionConfig profileGroups; /**< profileGroups
												   collection config */
	fiftyoneDegreesCollectionConfig profileOffsets; /**< ProfileOffsets
													collection config */
	fiftyoneDegreesCollectionConfig propertyTypes; /**< Property types collection
												   config */
	fiftyoneDegreesCollectionConfig graph; /**< Config for each graph */
} fiftyoneDegreesConfigIpi;

/**
 * Data set structure which contains the base dataset structure. This
 * acts as a wrapper and is used in the dataset structure for IP 
 * intelligence to be compatible with some of the common-cxx
 * APIs.
 */
typedef struct fiftyone_degrees_dataset_ipi_base_t {
	fiftyoneDegreesDataSetBase b; /**< Base structure members */
} fiftyoneDegreesDataSetIpiBase;

/**
 * Data set structure containing all the components used for IP intelligence.
 * This should predominantly be used through a #fiftyoneDegreesResourceManager
 * pointer to maintain a safe reference. If access the data set is needed then
 * a safe reference can be fetched and released with the
 * #fiftyoneDegreesDataSetIpiGet and #fiftyoneDegreesDataSetIpiRelease
 * methods.
 */
typedef struct fiftyone_degrees_dataset_ipi_t {
	fiftyoneDegreesDataSetIpiBase b; /**< Base data set */
	const fiftyoneDegreesDataSetIpiHeader header; /**< Dataset header */
	const fiftyoneDegreesConfigIpi config; /**< Copy of the configuration */
	fiftyoneDegreesCollection *strings; /**< Collection of all strings */
	fiftyoneDegreesCollection *components; /**< Collection of all components */
	fiftyoneDegreesList componentsList; /**< List of component items from the
										components collection */
	bool *componentsAvailable; /**< Array of flags indicating if there are
							   any properties available for the component with
							   the matching index in componentsList */
	uint32_t componentsAvailableCount; /**< Number of components with
									   properties */
	fiftyoneDegreesCollection *maps; /**< Collection data file maps */
	fiftyoneDegreesCollection *properties; /**< Collection data file properties
										   */
	fiftyoneDegreesCollection *values; /**< Collection data file values */
	fiftyoneDegreesCollection *profiles; /**< Collection data file profiles */
	fiftyoneDegreesCollection *graphs; /**< Collection of graph infos used to
									   create the array of graphs */
	fiftyoneDegreesCollection *profileGroups; /**< Collection of all profile 
											  groups where more than one 
											  profile is required with a weight
											  */
	fiftyoneDegreesCollection *propertyTypes; /**< Collection data file properties
											  */
	fiftyoneDegreesCollection *profileOffsets; /**< Collection of all offsets
											   to profiles in the profiles
											   collection */
	fiftyoneDegreesIpiCgArray* graphsArray; /**< Array of graphs from 
											collection */
} fiftyoneDegreesDataSetIpi;


/**
 * Backward compatibility typedef for ProfilePercentage.
 * @deprecated Use fiftyoneDegreesWeightedItem instead.
 */
typedef fiftyoneDegreesWeightedItem fiftyoneDegreesProfilePercentage;

/**
 * Backward compatibility typedef for IpiList.
 * @deprecated Use fiftyoneDegreesWeightedItemList instead.
 */
typedef fiftyoneDegreesWeightedItemList fiftyoneDegreesIpiList;

/**
 * Singular IP address result returned by a detection process method.
 */
typedef struct fiftyone_degrees_result_ipi_t {
	fiftyoneDegreesIpType type; /**< The version of the IP */
	fiftyoneDegreesIpiCgResult graphResult; /**< The result of
												 graph evaluation */
	fiftyoneDegreesIpAddress targetIpAddress; /**< The target IP address
											  to find a matching range for */
} fiftyoneDegreesResultIpi;

/**
 * Macro defining the common members of an Ipi result.
 */
#define FIFTYONE_DEGREES_RESULTS_IPI_MEMBERS \
	fiftyoneDegreesResultsBase b; \
	fiftyoneDegreesCollectionItem propertyItem; \
	fiftyoneDegreesIpiList values;

FIFTYONE_DEGREES_ARRAY_TYPE(
	fiftyoneDegreesResultIpi,
	FIFTYONE_DEGREES_RESULTS_IPI_MEMBERS)

/**
 * Array of Ipi results used to easily access and track the size of the
 * array.
 */
typedef fiftyoneDegreesResultIpiArray fiftyoneDegreesResultsIpi;

/**
 * Define the IP range structure
 */
#define FIFTYONE_DEGREES_IP_RANGE(v, s) \
typedef struct fiftyone_degrees_dataset_ip_v##v##_range_t { \
	byte start[s]; /**< the start of the range in byte array format */ \
	int32_t profileOffsetIndex; /**< The index of set the matching profile offset/group */ \
} fiftyoneDegreesIpv##v##Range;

/**
 * Define the ip roots structure
 */
FIFTYONE_DEGREES_IP_RANGE(4, 4)

/**
 * Define the ip nodes structure
 */
FIFTYONE_DEGREES_IP_RANGE(6, 16)

/**
 * Index of a profile in a profile groups item
 * This include the component Index to indicate in which component
 * it belong to.
 */
typedef struct fiftyone_degrees_combination_profile_index_t {
	int32_t componentIndex; /**< Index of the component based on components list */
	int32_t profileIndex; /**< Index of the profile index for the associated component */
} fiftyoneDegreesCombinationProfileIndex;

/**
 * Index of a profile in a profile groups item of a result
 * This include the component and result Indices to indicate in which
 * component the profile belong to in which result. 
 */
typedef struct fiftyone_degrees_result_profile_index_t {
	int16_t resultIndex;
	fiftyoneDegreesCombinationProfileIndex componentProfileIndex;
} fiftyoneDegreesResultProfileIndex;

/**
 * IP INTELLIGENCE CONFIGURATIONS
 */

/**
 * Configuration to be used where the data set is being created using a buffer
 * in memory and concepts like caching are not required. The concurrency
 * setting is ignored as there are no critical sections with this configuration.
 */
EXTERNAL_VAR fiftyoneDegreesConfigIpi fiftyoneDegreesIpiInMemoryConfig;

/**
 * Highest performance configuration. Loads all the data into memory and does
 * not maintain a connection to the source data file used to build the data
 * set. The concurrency setting is ignored as there are no critical sections
 * with this configuration.
 */
EXTERNAL_VAR fiftyoneDegreesConfigIpi fiftyoneDegreesIpiHighPerformanceConfig;

/**
 * Low memory configuration. A connection is maintained to the source data file
 * used to build the data set and used to load data into memory when required.
 * No caching is used resulting in the lowest memory footprint at the expense
 * of performance. The concurrency of each collection must be set to the
 * maximum number of concurrent operations to optimize file reads.
 */
EXTERNAL_VAR fiftyoneDegreesConfigIpi fiftyoneDegreesIpiLowMemoryConfig;

/**
 * Uses caching to balance memory usage and performance. A connection is
 * maintained to the source data file to load data into caches when required.
 * As the cache is loaded, memory will increase until the cache capacity is
 * reached. The concurrency of each collection must be set to the maximum
 * number of concurrent operations to optimize file reads. This is the default
 * configuration.
 */
EXTERNAL_VAR fiftyoneDegreesConfigIpi fiftyoneDegreesIpiBalancedConfig;

/**
 * Balanced configuration modified to create a temporary file copy of the
 * source data file to avoid locking the source data file.
 */
EXTERNAL_VAR fiftyoneDegreesConfigIpi fiftyoneDegreesIpiBalancedTempConfig;

/**
 * Default detection configuration. This configures the data set to not create
 * a temp file.
 */
EXTERNAL_VAR fiftyoneDegreesConfigIpi fiftyoneDegreesIpiDefaultConfig;


/**
 * EXTERNAL METHODS
 */

/**
 * Gets a safe reference to the IP Intelligencedata set from the resource manager.
 * Fetching through this method ensures that the data set it not freed or moved
 * during the time it is in use.
 * The data set returned by this method should be released with the
 * #fiftyoneDegreesDataSetIpiRelease method.
 * @param manager the resource manager containing a IP intelligence data set initialised
 * by one of the IP intelligence data set init methods
 * @return a fixed pointer to the data set in manager
 */
EXTERNAL fiftyoneDegreesDataSetIpi* fiftyoneDegreesDataSetIpiGet(
	fiftyoneDegreesResourceManager* manager);

/**
 * Release the reference to a data set returned by the
 * #fiftyoneDegreesDataSetIpiGet method. Doing so tell the resource manager
 * linked to the data set that it is no longer being used.
 * @param dataSet pointer to the data set to release
 */
EXTERNAL void fiftyoneDegreesDataSetIpiRelease(fiftyoneDegreesDataSetIpi* dataset);

/**
 * Gets the total size in bytes which will be allocated when intialising a
 * IP Intelligence resource and associated manager with the same parameters. If any of
 * the configuration options prevent the memory from being constant (i.e. more
 * memory may be allocated at process time) then zero is returned.
 * @param config configuration for the operation of the data set, or NULL if
 * default configuration is required
 * @param properties the properties that will be consumed from the data set, or
 * NULL if all available properties in the IP Intelligence data file should be available
 * for consumption
 * @param fileName the full path to a file with read permission that contains
 * the IP Intelligence data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the total number of bytes needed to initialise a IP Intelligence resource
 * and associated manager with the configuration provided or zero
 */
EXTERNAL size_t fiftyoneDegreesIpiSizeManagerFromFile(
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	const char* fileName,
	fiftyoneDegreesException* exception);

/**
 * Initialises the resource manager with an IP intelligence data set resource populated
 * from the IP Intelligence data file referred to by fileName. Configures the data set
 * to operate using the configuration set in detection, collection and
 * properties.
 * @param manager the resource manager to manager the share data set resource
 * @param config configuration for the operation of the data set, or NULL if
 * default configuration is required
 * @param properties the properties that will be consumed from the data set, or
 * NULL if all available properties in the IP Intelligence data file should be available
 * for consumption
 * @param fileName the full path to a file with read permission that contains
 * the IP Intelligence data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set resource assign to the
 * resource manager. Any value other than #FIFTYONE_DEGREES_STATUS_SUCCESS
 * means the data set was not created and the resource manager can not be used.
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesIpiInitManagerFromFile(
	fiftyoneDegreesResourceManager* manager,
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	const char* fileName,
	fiftyoneDegreesException* exception);

/**
 * Gets the total size in bytes which will be allocated when intialising a
 * IP Intelligence resource and associated manager with the same parameters. If any of
 * the configuration options prevent the memory from being constant (i.e. more
 * memory may be allocated at process time) then zero is returned.
 * @param config configuration for the operation of the data set, or NULL if
 * default configuration is required
 * @param properties the properties that will be consumed from the data set, or
 * NULL if all available properties in the IP Intelligence data file should be available
 * for consumption
 * @param memory pointer to continuous memory containing the IP intelligence data set
 * @param size the number of bytes that make up the IP intelligence data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the total number of bytes needed to initialise a IP Intelligence resource
 * and associated manager with the configuration provided or zero
 */
EXTERNAL size_t fiftyoneDegreesIpiSizeManagerFromMemory(
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	void* memory,
	fiftyoneDegreesFileOffset size,
	fiftyoneDegreesException* exception);

/**
 * Initialises the resource manager with a IP Intelligence data set resource populated
 * from the IP intelligence data set pointed to by the memory parameter. Configures the
 * data set to operate using the configuration set in detection and properties.
 * @param manager the resource manager to manager the share data set resource
 * @param config configuration for the operation of the data set, or NULL if
 * default configuration is required
 * @param properties the properties that will be consumed from the data set, or
 * NULL if all available properties in the IP intelligence data file should be available
 * for consumption
 * @param memory pointer to continuous memory containing the IP Intelligence data set
 * @param size the number of bytes that make up the IP Intelligence data set
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set resource assign to the
 * resource manager. Any value other than #FIFTYONE_DEGREES_STATUS_SUCCESS
 * means the data set was not created and the resource manager can not be used.
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesIpiInitManagerFromMemory(
	fiftyoneDegreesResourceManager* manager,
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	void* memory,
	fiftyoneDegreesFileOffset size,
	fiftyoneDegreesException* exception);

/**
 * Read a profile groups item from the file collection provided and 
 * store in the data pointer. This method is used when creating a collection
 * from file.
 * @param file collection to read from
 * @param offset of the profile groups in the collection
 * @param data to store the resulting profile groups in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the component allocated within the data structure
 */
EXTERNAL void* fiftyoneDegreesProfileCombinationReadFromFile(
	const fiftyoneDegreesCollectionFile* file,
	uint32_t offset,
	fiftyoneDegreesData* data,
	fiftyoneDegreesException* exception);

/**
 * Reload the data set being used by the resource manager using the data file
 * location specified. When initialising the data, the configuration that
 * manager was first created with is used.
 *
 * If the new data file is successfully initialised, the current data set is
 * replaced The old data will remain in memory until the last
 * #fiftyoneDegreesResultsIpi which contain a reference to it are released.
 *
 * This method is defined by the #FIFTYONE_DEGREES_DATASET_RELOAD macro.
 * @param manager pointer to the resource manager to reload the data set for
 * @param fileName path to the new data file
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set reload. Any value other than
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded
 * correctly
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesIpiReloadManagerFromFile(
	fiftyoneDegreesResourceManager* manager,
	const char* fileName,
	fiftyoneDegreesException* exception);

/**
 * Reload the data set being used by the resource manager using a data file
 * loaded into contiguous memory. When initialising the data, the configuration
 * that manager was first created with is used.
 *
 * If the data passed in is successfully initialised, the current data set is
 * replaced The old data will remain in memory until the last
 * #fiftyoneDegreesResultsIpi which contain a reference to it are released.
 *
 * This method is defined by the #FIFTYONE_DEGREES_DATASET_RELOAD macro.
 * @param manager pointer to the resource manager to reload the data set for
 * @param source pointer to the memory location where the new data file is
 *               stored
 * @param length of the data in memory
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set reload. Any value other than
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded
 * correctly
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesIpiReloadManagerFromMemory(
	fiftyoneDegreesResourceManager* manager,
	void* source,
	fiftyoneDegreesFileOffset length,
	fiftyoneDegreesException* exception);

/**
 * Reload the data set being used by the resource manager using the data file
 * location which was used when the manager was created. When initialising the
 * data, the configuration that manager was first created with is used.
 *
 * If the new data file is successfully initialised, the current data set is
 * replaced The old data will remain in memory until the last
 * #fiftyoneDegreesResultsIpi which contain a reference to it are released.
 *
 * This method is defined by the #FIFTYONE_DEGREES_DATASET_RELOAD macro.
 * @param manager pointer to the resource manager to reload the data set for
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the status associated with the data set reload. Any value other than
 * #FIFTYONE_DEGREES_STATUS_SUCCESS means the data set was not reloaded
 * correctly
 */
EXTERNAL fiftyoneDegreesStatusCode  
fiftyoneDegreesIpiReloadManagerFromOriginalFile(
	fiftyoneDegreesResourceManager* manager,
	fiftyoneDegreesException* exception);

/**
 * Allocates a results structure containing a reference to the IP Intelligence
 * data set managed by the resource manager provided. The referenced data set
 * will be kept active until the results structure is freed.
 * There can only be one result for per input IP address
 * @param manager pointer to the resource manager which manages a IP 
 * Intelligence data set
 * @return newly created results structure
 */
EXTERNAL fiftyoneDegreesResultsIpi* fiftyoneDegreesResultsIpiCreate(
	fiftyoneDegreesResourceManager* manager);

/**
 * Frees the results structure created by the
 * #fiftyoneDegreesResultsIpiCreate method. When freeing, the reference to
 * the IP Intelligence data set resource is released.
 * @param results pointer to the results structure to release
 */
EXTERNAL void fiftyoneDegreesResultsIpiFree(
	fiftyoneDegreesResultsIpi* results);

/**
 * Process a single byte array format IP Address and populate the IP range 
 * offset in the results structure. The result IP type will need to be checked
 * to determined the version of IP being processed.
 * @param results preallocated results structure to populate
 * @param ipAddress byte array to process
 * @param ipAddressLength of the IP address byte array
 * @param type of the ip
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL void fiftyoneDegreesResultsIpiFromIpAddress(
	fiftyoneDegreesResultsIpi* results,
	const unsigned char* ipAddress,
	size_t ipAddressLength,
	fiftyoneDegreesIpType type,
	fiftyoneDegreesException* exception);

/**
 * Process a single IP Address and populate the IP range offset in the results
 * structure.
 * @param results preallocated results structure to populate
 * @param ipAddress string to process
 * @param ipAddressLength of the ipAddress string
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL void fiftyoneDegreesResultsIpiFromIpAddressString(
	fiftyoneDegreesResultsIpi* results,
	const char* ipAddress,
	size_t ipAddressLength,
	fiftyoneDegreesException* exception);

/**
 * Processes the evidence value pairs in the evidence collection and
 * populates the result in the results structure.
 * @param results preallocated results structure to populate containing a
 *                pointer to an initialised resource manager
 * @param evidence to process containing parsed or unparsed values
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL void fiftyoneDegreesResultsIpiFromEvidence(
	fiftyoneDegreesResultsIpi* results,
	fiftyoneDegreesEvidenceKeyValuePairArray* evidence,
	fiftyoneDegreesException* exception);

/**
 * Gets whether or not the results provided contain valid values for the
 * property index provided.
 * @param results pointer to the results to check
 * @param requiredPropertyIndex index in the required properties of the
 * property to check for values of
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return true if there are valid values in the results for the property index
 * provided
 */
EXTERNAL bool fiftyoneDegreesResultsIpiGetHasValues(
	fiftyoneDegreesResultsIpi* results,
	int requiredPropertyIndex,
	fiftyoneDegreesException* exception);

/**
 * Gets the reason why a results does not contain valid values for a given
 * property. 
 * @param results pointer to the results to check
 * @param requiredPropertyIndex index in the required properties of the
 * property to check for values of
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return enum indicating why a valid value cannot be returned by the results
 */
EXTERNAL fiftyoneDegreesResultsNoValueReason fiftyoneDegreesResultsIpiGetNoValueReason(
	fiftyoneDegreesResultsIpi* results,
	int requiredPropertyIndex,
	fiftyoneDegreesException* exception);

/**
 * Gets a fuller description of the reason why a value is missing.
 * @param reason enum of the reason for the missing value
 * @return full description for the reason
 */
EXTERNAL const char* fiftyoneDegreesResultsIpiGetNoValueReasonMessage(
	fiftyoneDegreesResultsNoValueReason reason);

/**
 * Populates the list of values in the results instance with value structure
 * instances associated with the required property index. When the results 
 * are released then the value items will be released. There is no need for
 * the caller to release the collection item returned. The 
 * fiftyoneDegreesResultsIpiGetValueString method should be used to get
 * the string representation of the value.
 * @param results pointer to the results structure to release
 * @param requiredPropertyIndex
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to the first value item 
 */
EXTERNAL const fiftyoneDegreesProfilePercentage* fiftyoneDegreesResultsIpiGetValues(
	fiftyoneDegreesResultsIpi* results,
	int requiredPropertyIndex,
	fiftyoneDegreesException* exception);

/**
 * Adds to builder the values associated in the results for the property name.
 * @param results pointer to the results structure to release
 * @param propertyName name of the property to be used with the values
 * @param builder string builder to fill with values
 * @param separator string to be used to separate multiple values if available
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 */
EXTERNAL void fiftyoneDegreesResultsIpiAddValuesString(
	fiftyoneDegreesResultsIpi* results,
	const char* propertyName,
	fiftyoneDegreesStringBuilder *builder,
	const char* separator,
	fiftyoneDegreesException* exception);

/**
 * Sets the buffer the values associated in the results for the property name.
 * @param results pointer to the results structure to release
 * @param propertyName name of the property to be used with the values
 * @param buffer character buffer allocated by the caller
 * @param bufferLength of the character buffer
 * @param separator string to be used to separate multiple values if available
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the number of characters available for values. May be larger than
 * bufferLength if the buffer is not long enough to return the result.
 */
EXTERNAL size_t fiftyoneDegreesResultsIpiGetValuesString(
	fiftyoneDegreesResultsIpi* results,
	const char* propertyName,
	char* buffer,
	size_t bufferLength,
	const char* separator,
	fiftyoneDegreesException* exception);

/**
 * Sets the buffer the values associated in the results for the property name.
 * @param results pointer to the results structure to release
 * @param requiredPropertyIndex required property index of for the values
 * @param buffer character buffer allocated by the caller
 * @param bufferLength of the character buffer
 * @param separator string to be used to separate multiple values if available
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return the number of characters available for values. May be larger than
 * bufferLength if the buffer is not long enough to return the result.
 */
EXTERNAL size_t fiftyoneDegreesResultsIpiGetValuesStringByRequiredPropertyIndex(
	fiftyoneDegreesResultsIpi* results,
	const int requiredPropertyIndex,
	char* buffer,
	size_t bufferLength,
	const char* separator,
	fiftyoneDegreesException* exception);

/**
 * Get the network id string from the single result provided. This contains
 * profile ids for all components and their percentages for the matched
 * IP range, delimited by ':'. The profile id percentage pairs are concatenated
 * with the separator character '|'.
 * @param results pointer to the results
 * @param result pointer to the result to get the network id of
 * @param destination pointer to the memory to write the characters to
 * @param size amount of memory allocated to destination
 * @param componentProfileIndex the combination profile index to start
 * constructing the network ID from. Negative values indicate nothing
 * to fetch.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the combination profile index to start from in the next call for
 * fetching the remaining of the network ID. All indices will be -1s there
 * is no more to fetch. The returned format does not contain leading or
 * trailing separator so the separator will need to be added explicitly
 * if more than one call  is required.
 */
EXTERNAL fiftyoneDegreesCombinationProfileIndex 
fiftyoneDegreesIpiGetNetworkIdFromResult(
	fiftyoneDegreesResultsIpi* results,
	fiftyoneDegreesResultIpi* result,
	char* destination,
	size_t size,
	fiftyoneDegreesCombinationProfileIndex componentProfileIndex,
	fiftyoneDegreesException* exception);

/**
 * Get the network id string from the results provided. This contains
 * profile ids for all components and their percentages for the matched
 * IP range, delimited by ':'. The profile id percentage pairs are concatenated
 * with the separator character '|'.
 * @param results pointer to the results
 * @param destination pointer to the memory to write the characters to
 * @param size amount of memory allocated to destination
 * @param resultProfileIndex the profile index to start constructing the network
 * ID from. Negative values indicate nothing to fetch.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the result profile index to start from in the next call for
 * fetching the remaining of the network ID. All indices will be -1s there
 * is no more to fetch. The returned format does not contain leading or
 * trailing separator so the separator will need to be added explicitly
 * if more than one call  is required.
 */
EXTERNAL fiftyoneDegreesResultProfileIndex
fiftyoneDegreesIpiGetNetworkIdFromResults(
	fiftyoneDegreesResultsIpi* results,
	char* destination,
	size_t size,
	fiftyoneDegreesResultProfileIndex resultProfileIndex,
	fiftyoneDegreesException* exception);

/**
 * Iterates over the profiles in the data set calling the callback method for
 * any profiles that contain the property and value provided.
 * This currently not applicable for properties 'IpRangeStart', 'IpRangeEnd',
 * 'AverageLocation', 'LocationBoundSouthEast', 'LocationBoundNortWest' as
 * these data are not stored in the profiles collection.
 * @param manager the resource manager containing a IP Intelligence data set initialised
 * by one of the IP Intelligence data set init methods
 * @param propertyName name of the property which the value relates to
 * @param valueName name of the property value which the profiles must contain
 * @param state pointer passed to the callback method
 * @param callback method called when a matching profile is found
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number of matching profiles iterated over
 */
EXTERNAL uint32_t fiftyoneDegreesIpiIterateProfilesForPropertyAndValue(
	fiftyoneDegreesResourceManager* manager,
	const char* propertyName,
	const char* valueName,
	void* state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException* exception);

/**
 * Get the ipaddress string from the collection item. This should
 * be used on the item returned from #fiftyoneDegreesResultsIpiGetValues
 * where the property is 'RangeStart', 'RangeEnd'.
 * @param item the collection item pointing to the strings item in
 * strings collection
 * @param type the verion of IP which can be check from result
 * @param buffer the preallocated buffer to hold the returned string
 * @param bufferLength the number of bytes allocated for the buffer
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number of characters added to the buffer
 */
EXTERNAL size_t fiftyoneDegreesIpiGetIpAddressAsString(
	const fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesIpType type,
	char *buffer,
	uint32_t bufferLength,
	fiftyoneDegreesException *exception);

/**
 * @}
 */

#endif
