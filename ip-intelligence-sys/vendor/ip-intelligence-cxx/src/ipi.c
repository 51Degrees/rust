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

#include "ipi.h"
#include "fiftyone.h"
#include "common-cxx/config.h"
#include "constantsIpi.h"
#include "common-cxx/collectionKeyTypes.h"
#include "ip-graph-cxx/graph.h"

MAP_TYPE(Collection)

/**
 * GENERAL MACROS TO IMPROVE READABILITY
 */

/** Offset used for a null profile. */
#define NULL_PROFILE_OFFSET UINT32_MAX
/** Offset does not have value. */
#define NULL_VALUE_OFFSET UINT32_MAX
/** Dynamic component */
#define DYNAMIC_COMPONENT_OFFSET UINT32_MAX

/** Default value and percentage separator */
#define DEFAULT_VALUE_PERCENTAGE_SEPARATOR ":"
/** Default values separator */
#define DEFAULT_VALUES_SEPARATOR "|"

#define COMPONENT(d, i) i < d->componentsList.count ? \
(Component*)d->componentsList.items[i].data.ptr : NULL

#define MAX_CONCURRENCY(t) if (config->t.concurrency > concurrency) { \
concurrency = config->t.concurrency; }

#define COLLECTION_CREATE_MEMORY(t) \
dataSet->t = CollectionCreateFromMemory( \
reader, \
dataSet->header.t); \
if (dataSet->t == NULL) { \
	return INVALID_COLLECTION_CONFIG; \
}

#define COLLECTION_CREATE_FILE(t,f) \
dataSet->t = CollectionCreateFromFile( \
	file, \
	&dataSet->b.b.filePool, \
	&dataSet->config.t, \
	dataSet->header.t, \
	f); \
if (dataSet->t == NULL) { \
	return INVALID_COLLECTION_CONFIG; \
}

/** 
 * Get min/max values with header guards to prevent redefinition warnings
 * when amalgamated with system headers (e.g., macOS sys/param.h)
 */
/** Get min value */
#ifndef MIN
#define MIN(a,b) a < b ? a : b
#endif /* MIN */
/** Get max value */
#ifndef MAX
#define MAX(a,b) a > b ? a : b
#endif /* MAX */

/**
 * PRIVATE DATA STRUCTURES
 */

/**
 * Used to pass a data set pointer and an exception to methods that require a
 * callback method and a void pointer for state used by the callback method.
 */
typedef struct state_with_exception_t {
	void* state; /* Pointer to the data set or other state information */
	Exception* exception; /* Pointer to the exception structure */
} stateWithException;

typedef struct state_with_weighting_t {
	void* subState; /* Pointer to a data set or other information */
	uint16_t rawWeighting;
} stateWithWeighting;

/**
 * Used to pass a state together with an unique header index which
 * might be used to compared against evidence.
 */
typedef struct state_with_unique_header_index_t {
	void* subState; /* Pointer to the data set or other state information */
	uint32_t headerIndex; /* An unique header index to use */
} stateWithUniqueHeaderIndex;

/**
 * Used to represent the structure within a profile groups item
 */
#pragma pack(push, 2)
typedef struct profile_combination_component_index_t {
	uint16_t index; /* Index to the first profile of the component
					in the profiles list */
	uint16_t count; /* The number of profiles presents for that
					component */
} componentIndex;
#pragma pack(pop)

#pragma pack(push, 1)
typedef struct offset_percentage_t {
	uint32_t offset; /* Offset to a profiles collection item */
	uint16_t rawWeighting; /* The weight of the item in the matched IP range, out of 65535 */
} offsetPercentage;
#pragma pack(pop)

/**
 * All profile weightings in a groups should add up to exactly this number.
 */
static const uint16_t FULL_RAW_WEIGHTING = 0xFFFFU;

/**
 * PRESET IP INTELLIGENCE CONFIGURATIONS
 */

/* The expected version of the data file */
#define FIFTYONE_DEGREES_IPI_TARGET_VERSION_MAJOR 4
#define FIFTYONE_DEGREES_IPI_TARGET_VERSION_MINOR 5

#undef FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY
#define FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY true
fiftyoneDegreesConfigIpi fiftyoneDegreesIpiInMemoryConfig = {
	{FIFTYONE_DEGREES_CONFIG_DEFAULT_WITH_INDEX},
	{0,0,0}, // Strings
	{0,0,0}, // Components
	{0,0,0}, // Maps
	{0,0,0}, // Properties
	{0,0,0}, // Values
	{0,0,0}, // Profiles
	{0,0,0}, // Graphs
	{0,0,0}, // ProfileGroups
	{0,0,0}, // PropertyTypes
	{0,0,0}, // ProfileOffsets
	{0,0,0}  // Graph
};
#undef FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY
#define FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY \
FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY_DEFAULT

fiftyoneDegreesConfigIpi fiftyoneDegreesIpiHighPerformanceConfig = {
	{ FIFTYONE_DEGREES_CONFIG_DEFAULT_WITH_INDEX },
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Strings
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Components
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Maps
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Properties
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Values
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Profiles
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Graphs
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // ProfileGroups
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // PropertyTypes
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // ProfileOffsets
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }  // Graph
};

fiftyoneDegreesConfigIpi fiftyoneDegreesIpiLowMemoryConfig = {
	{ FIFTYONE_DEGREES_CONFIG_DEFAULT_NO_INDEX },
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Strings
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Components
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Maps
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Properties
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Values
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Profiles
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // Graphs
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // ProfileGroups
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // PropertyTypes
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, // ProfileOffsets
	{ false, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }  // Graph
};

#define FIFTYONE_DEGREES_IPI_CONFIG_BALANCED \
{ FIFTYONE_DEGREES_CONFIG_DEFAULT_WITH_INDEX }, \
{ FIFTYONE_DEGREES_STRING_LOADED, FIFTYONE_DEGREES_STRING_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Strings */ \
{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Components */ \
{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Maps */ \
{ FIFTYONE_DEGREES_PROPERTY_LOADED, FIFTYONE_DEGREES_PROPERTY_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Properties */ \
{ FIFTYONE_DEGREES_VALUE_LOADED, FIFTYONE_DEGREES_VALUE_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Values */ \
{ FIFTYONE_DEGREES_PROFILE_LOADED, FIFTYONE_DEGREES_PROFILE_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Profiles */ \
{ FIFTYONE_DEGREES_IP_GRAPHS_LOADED, FIFTYONE_DEGREES_IP_GRAPHS_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Graphs */ \
{ FIFTYONE_DEGREES_PROFILE_GROUPS_LOADED, FIFTYONE_DEGREES_PROFILE_GROUPS_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* ProfileGroups */ \
{ FIFTYONE_DEGREES_PROPERTY_LOADED, FIFTYONE_DEGREES_PROPERTY_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Property Types */ \
{ FIFTYONE_DEGREES_PROFILE_LOADED, FIFTYONE_DEGREES_PROFILE_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* ProfileOffsets */ \
{ FIFTYONE_DEGREES_IP_GRAPH_LOADED, FIFTYONE_DEGREES_IP_GRAPH_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY } /* Graph */

fiftyoneDegreesConfigIpi fiftyoneDegreesIpiBalancedConfig = {
	FIFTYONE_DEGREES_IPI_CONFIG_BALANCED
};

fiftyoneDegreesConfigIpi fiftyoneDegreesIpiDefaultConfig = {
	FIFTYONE_DEGREES_IPI_CONFIG_BALANCED
};

#undef FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE
#define FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE true

// BalancedTemp config with reuseTempFile enabled to avoid copying the
// data file for every test. This significantly improves performance on
// Windows where file I/O is slower.
fiftyoneDegreesConfigIpi fiftyoneDegreesIpiBalancedTempConfig = {
	{
		FIFTYONE_DEGREES_CONFIG_ALL_IN_MEMORY, /* allInMemory */
		true, /* usesUpperPrefixedHeaders */
		false, /* freeData */
		true, /* useTempFile */
		true, /* reuseTempFile - ENABLED for better performance */
		NULL, /* tempDirs */
		0, /* tempDirCount */
		true /* propertyValueIndex */
	},
	{ FIFTYONE_DEGREES_STRING_LOADED, FIFTYONE_DEGREES_STRING_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Strings */
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Components */
	{ true, 0, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Maps */
	{ FIFTYONE_DEGREES_PROPERTY_LOADED, FIFTYONE_DEGREES_PROPERTY_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Properties */
	{ FIFTYONE_DEGREES_VALUE_LOADED, FIFTYONE_DEGREES_VALUE_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Values */
	{ FIFTYONE_DEGREES_PROFILE_LOADED, FIFTYONE_DEGREES_PROFILE_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Profiles */
	{ FIFTYONE_DEGREES_IP_GRAPHS_LOADED, FIFTYONE_DEGREES_IP_GRAPHS_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Graphs */
	{ FIFTYONE_DEGREES_PROFILE_GROUPS_LOADED, FIFTYONE_DEGREES_PROFILE_GROUPS_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* ProfileGroups */
	{ FIFTYONE_DEGREES_PROPERTY_LOADED, FIFTYONE_DEGREES_PROPERTY_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* Property Types */
	{ FIFTYONE_DEGREES_PROFILE_LOADED, FIFTYONE_DEGREES_PROFILE_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY }, /* ProfileOffsets */
	{ FIFTYONE_DEGREES_IP_GRAPH_LOADED, FIFTYONE_DEGREES_IP_GRAPH_CACHE_SIZE, FIFTYONE_DEGREES_CACHE_CONCURRENCY } /* Graph */
};
#undef FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE
#define FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE \
FIFTYONE_DEGREES_CONFIG_USE_TEMP_FILE_DEFAULT

/**
 * IP INTELLIGENCE METHODS
 */

static void resultIpiReset(ResultIpi* result) {
	memset(result->targetIpAddress.value, 0, FIFTYONE_DEGREES_IPV6_LENGTH);
	result->targetIpAddress.type = IP_TYPE_INVALID;
}

static int compareIpAddresses(
	const byte* address1,
	const byte* address2,
	int length) {
	for (int i = 0; i < length; i++) {
		const int difference = (int)address1[i] - (int)address2[i];
		if (difference != 0) return difference;
	}
	return 0;
}

static CollectionKeyType CollectionKeyType_Ipv4Range = {
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_IPV4_RANGE,
	sizeof(Ipv4Range),
	NULL,
};
static CollectionKeyType CollectionKeyType_Ipv6Range = {
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_IPV6_RANGE,
	sizeof(Ipv6Range),
	NULL,
};

static int compareToIpv4Range(
	const void * const state,
	const Item* item,
	long curIndex,
	Exception* exception) {
	int result = 0;
	const fiftyoneDegreesIpAddress target = *((const fiftyoneDegreesIpAddress *)state);
	// We will terminate if IP address is within the range between the current item and the next item
	const int tempResult = compareIpAddresses(((Ipv4Range*)item->data.ptr)->start, target.value, FIFTYONE_DEGREES_IPV4_LENGTH);
	if (tempResult < 0) {
		Item nextItem;
		DataReset(&nextItem.data);
		if ((uint32_t)curIndex + 1 < item->collection->count) {
			const CollectionKey curKey = {
				(uint32_t)++curIndex,
				&CollectionKeyType_Ipv4Range,
			};
			if (item->collection->get(
				item->collection,
				&curKey,
				&nextItem,
				exception) != NULL && EXCEPTION_OKAY) {
				if (compareIpAddresses(
					((Ipv4Range*)nextItem.data.ptr)->start,
					target.value,
					FIFTYONE_DEGREES_IPV4_LENGTH) <= 0) {
					result = -1;
				}
				COLLECTION_RELEASE(item->collection, &nextItem);
			}
		}
	}
	else if (tempResult > 0 && curIndex > 0) {
		// The IP address is out of range
		// NOTE: If the current index is 0
		// There is no more item lower so return the current
		result = 1;
	}
	return result;
}

static int compareToIpv6Range(
	const void * const state,
	const Item * const item,
	long curIndex,
	Exception* exception) {
	int result = 0;
	const fiftyoneDegreesIpAddress target = *((fiftyoneDegreesIpAddress*)state);
	// We will terminate if IP address is within the range between the current item and the next item
	const int tempResult = compareIpAddresses(((Ipv6Range*)item->data.ptr)->start, target.value, FIFTYONE_DEGREES_IPV6_LENGTH);
	if (tempResult < 0) {
		Item nextItem;
		DataReset(&nextItem.data);
		if ((uint32_t)curIndex + 1 < item->collection->count) {
			const CollectionKey curKey = {
				(uint32_t)++curIndex,
				&CollectionKeyType_Ipv6Range,
			};
			if (item->collection->get(
				item->collection,
				&curKey,
				&nextItem,
				exception) != NULL && EXCEPTION_OKAY) {

				if (compareIpAddresses(((Ipv6Range*)nextItem.data.ptr)->start, target.value, FIFTYONE_DEGREES_IPV6_LENGTH) <= 0) {
					// The IP address is not within the range
					result = -1;
				}
				COLLECTION_RELEASE(item->collection, &nextItem);
			}
		}
	}
	else if (tempResult > 0 && curIndex > 0) {
		// The IP address is out of range
		// NOTE: There is no more item lower
		// so return the current
		result = 1;
	}
	return result;
}

static void setResultFromIpAddress(
	ResultIpi* const result,
	const DataSetIpi* const dataSet,
	byte componentId,
	Exception* const exception) {
	const fiftyoneDegreesIpiCgResult graphResult = fiftyoneDegreesIpiGraphEvaluate(
		dataSet->graphsArray,
		componentId,
		result->targetIpAddress, 
		exception);
	if (graphResult.rawOffset != NULL_PROFILE_OFFSET && EXCEPTION_OKAY) {
		result->graphResult = graphResult;
	}
}

/**
 * DATA INITIALISE AND RESET METHODS
 */

static void resetDataSet(DataSetIpi* dataSet) {
	DataSetReset(&dataSet->b.b);
	ListReset(&dataSet->componentsList);
	dataSet->componentsAvailable = NULL;
	dataSet->components = NULL;
	dataSet->maps = NULL;
	dataSet->graphs = NULL;
	dataSet->profileGroups = NULL;
	dataSet->profileOffsets = NULL;
	dataSet->profiles = NULL;
	dataSet->properties = NULL;
	dataSet->propertyTypes = NULL;
	dataSet->strings = NULL;
	dataSet->values = NULL;
	dataSet->graphsArray = NULL;
}

static void freeDataSet(void* dataSetPtr) {
	DataSetIpi* dataSet = (DataSetIpi*)dataSetPtr;

	// Free the common data set fields.
	DataSetFree(&dataSet->b.b);

	// Free the resources associated with the graphs.
	if (dataSet->graphsArray) {
		fiftyoneDegreesIpiGraphFree(dataSet->graphsArray);
	}

	// Free the memory used for the lists and collections.
	ListFree(&dataSet->componentsList);
	Free(dataSet->componentsAvailable);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->strings);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->components);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->properties);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->maps);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->values);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->profiles);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->propertyTypes);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->graphs);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->profileOffsets);
	FIFTYONE_DEGREES_COLLECTION_FREE(dataSet->profileGroups);
	
	// Finally free the memory used by the resource itself as this is always
	// allocated within the IP Intelligence init manager method.
	Free(dataSet);
}

static long initGetHttpHeaderString(
	void *state,
	uint32_t index,
	Item *nameItem) {
	const DataSetIpi *dataSet =
		(DataSetIpi*)((stateWithException*)state)->state;
	Exception *exception = ((stateWithException*)state)->exception;
	uint32_t i = 0, c = 0;
	Component *component = COMPONENT(dataSet, c);
	c++;
	while (component != NULL) {
		if (index < i + component->keyValuesCount) {
			const ComponentKeyValuePair *keyValue =
				ComponentGetKeyValuePair(
					component,
					(uint16_t)(index - i),
					exception);
			nameItem->collection = NULL;
			StoredBinaryValueGet(
				dataSet->strings,
				keyValue->key,
				FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // key is string
				nameItem,
				exception);
			return keyValue->key;
		}
		i += component->keyValuesCount;
		component = COMPONENT(dataSet, c);
		c++;
	}
	return -1;
}

static const String* initGetPropertyString(
	void* state,
	uint32_t index,
	Item* item) {
	const String* name = NULL;
	Item propertyItem;
	Property* property;
	const DataSetIpi* dataSet = (DataSetIpi*)((stateWithException*)state)->state;
	Exception* exception = ((stateWithException*)state)->exception;
	const uint32_t propertiesCount = CollectionGetCount(dataSet->properties);
	DataReset(&item->data);
	if (index < propertiesCount) {
		DataReset(&propertyItem.data);
		item->collection = NULL;
		item->handle = NULL;
		const CollectionKey indexKey = {
			index,
			CollectionKeyType_Property,
		};
		property = (Property*)dataSet->properties->get(
			dataSet->properties,
			&indexKey,
			&propertyItem,
			exception);
		if (property != NULL && EXCEPTION_OKAY) {
			name = PropertyGetName(
				dataSet->strings,
				property,
				item,
				exception);
			if (EXCEPTION_OKAY) {
				COLLECTION_RELEASE(dataSet->properties, &propertyItem);
			}
		}
	}
	return name;
}

static StatusCode initComponentsAvailable(
	DataSetIpi* dataSet,
	Exception* exception) {
	uint32_t i;
	Property* property;
	Item item;
	DataReset(&item.data);

	for (i = 0;
		i < dataSet->b.b.available->count;
		i++) {
		property = PropertyGet(
			dataSet->properties,
			dataSet->b.b.available->items[i].propertyIndex,
			&item,
			exception);
		if (property == NULL || EXCEPTION_FAILED) {
			return COLLECTION_FAILURE;
		}
		dataSet->componentsAvailable[property->componentIndex] = true;
		COLLECTION_RELEASE(dataSet->properties, &item);
	}

	// Count the number of components with available properties. Needed when
	// creating results to allocate sufficient capacity for all the components.
	dataSet->componentsAvailableCount = 0;
	for (i = 0; i < dataSet->componentsList.count; i++) {
		if (dataSet->componentsAvailable[i]) {
			dataSet->componentsAvailableCount++;
		}
	}

	return SUCCESS;
}

static int findPropertyIndexByName(
	Collection *properties,
	Collection *strings,
	char *name,
	Exception *exception) {
	int index;
	bool found = false;
	Property *property;
	const String *propertyName;
	Item propertyItem, nameItem;
	const int count = CollectionGetCount(properties);
	DataReset(&propertyItem.data);
	DataReset(&nameItem.data);
	for (index = 0; index < count && found == false; index++) {
		property = PropertyGet(
			properties,
			index,
			&propertyItem,
			exception);
		if (property != NULL &&
			EXCEPTION_OKAY) {
			propertyName = PropertyGetName(
				strings,
				property,
				&nameItem,
				exception);
			if (propertyName != NULL && EXCEPTION_OKAY) {
				if (StringCompare(name, &propertyName->value) == 0) {
					found = true;
				}
				COLLECTION_RELEASE(strings, &nameItem);
			}
			COLLECTION_RELEASE(properties, &propertyItem);
		}
	}
	return found ? index : -1;
}

static void initGetEvidencePropertyRelated(
	DataSetIpi* const dataSet,
	PropertyAvailable* const availableProperty,
	EvidenceProperties* const evidenceProperties,
	int* const count,
	char* const suffix,
	Exception* const exception) {
	const Property* property;
	const String* name;
	const String* availableName = (String*)availableProperty->name.data.ptr;
	const int requiredLength = ((int)strlen(suffix)) + availableName->size - 1;
	Item propertyItem, nameItem;
	DataReset(&propertyItem.data);
	DataReset(&nameItem.data);
	const int propertiesCount = CollectionGetCount(dataSet->properties);
	for (int propertyIndex = 0; 
		propertyIndex < propertiesCount && EXCEPTION_OKAY; 
		propertyIndex++) {
		property = PropertyGet(
			dataSet->properties,
			propertyIndex,
			&propertyItem,
			exception);
		if (property != NULL && EXCEPTION_OKAY) {
			name = &StoredBinaryValueGet(
				dataSet->strings,
				property->nameOffset,
				FIFTYONE_DEGREES_PROPERTY_VALUE_TYPE_STRING, // name is string
				&nameItem,
				exception)->stringValue;
			if (name != NULL && EXCEPTION_OKAY) {
				if (requiredLength == name->size -1 &&
					// Check that the available property matches the start of
					// the possible related property.
					StringCompareLength(
						&availableName->value,
						&name->value,
						(size_t)availableName->size - 1) == 0 && 
					// Check that the related property has a suffix that 
					// matches the one provided to the method.
					StringCompare(
						&name->value + availableName->size - 1, 
						suffix) == 0) {
					if (evidenceProperties != NULL) {
						evidenceProperties->items[*count] = propertyIndex;
					}
					(*count)++;
				}
				COLLECTION_RELEASE(dataSet->strings, &nameItem);
			}
			COLLECTION_RELEASE(dataSet->properties, &propertyItem);
		}
	}
}

static uint32_t initGetEvidenceProperties(
	void* state,
	fiftyoneDegreesPropertyAvailable* availableProperty,
	fiftyoneDegreesEvidenceProperties* evidenceProperties) {
	int count = 0;
	DataSetIpi* dataSet =
		(DataSetIpi*)((stateWithException*)state)->state;
	Exception* exception = ((stateWithException*)state)->exception;

	// Any properties that have a suffix of JavaScript and are associated with
	// an available property should also be added. These are used to gather
	// evidence from JavaScript that might impact the value returned.
	initGetEvidencePropertyRelated(
		dataSet,
		availableProperty,
		evidenceProperties,
		&count,
		"JavaScript",
		exception);

	return (uint32_t)count;
}

static StatusCode initPropertiesAndHeaders(
	DataSetIpi* dataSet,
	PropertiesRequired* properties,
	Exception* exception) {
	stateWithException state;
	state.state = (void*)dataSet;
	state.exception = exception;
	StatusCode status =  DataSetInitProperties(
		&dataSet->b.b,
		properties,
		&state,
		initGetPropertyString,
		initGetEvidenceProperties);
	if (status != SUCCESS) {
		return status;
	}

	status = DataSetInitHeaders(
		&dataSet->b.b,
		&state,
		initGetHttpHeaderString,
		exception);
	if (status != SUCCESS) {
		return status;
	}

	return status;
}

static StatusCode readHeaderFromMemory(
	MemoryReader* reader,
	const DataSetIpiHeader* header) {

	// Copy the bytes that make up the dataset header.
	if (memcpy(
		(void*)header,
		(const void*)reader->current,
		sizeof(DataSetIpiHeader)) != header) {
		return CORRUPT_DATA;
	}

	// Move the current pointer to the next data structure.
	return MemoryAdvance(reader, sizeof(DataSetIpiHeader)) == true ?
		SUCCESS : CORRUPT_DATA;
}

static StatusCode checkVersion(DataSetIpi* dataSet) {
	if (!(dataSet->header.versionMajor ==
		FIFTYONE_DEGREES_IPI_TARGET_VERSION_MAJOR &&
		dataSet->header.versionMinor ==
		FIFTYONE_DEGREES_IPI_TARGET_VERSION_MINOR)) {
		return INCORRECT_VERSION;
	}
	return SUCCESS;
}

static void initDataSetPost(
	DataSetIpi* dataSet,
	Exception* exception) {
	
	// Initialise the components lists
	ComponentInitList(
		dataSet->components,
		&dataSet->componentsList,
		dataSet->header.components.count,
		exception);
	if (EXCEPTION_FAILED) {
		return;
	}

	// Initialise the components which have required properties.
	dataSet->componentsAvailable = Malloc(
		sizeof(bool) * dataSet->componentsList.count);
	if (dataSet->componentsAvailable == NULL) {
		EXCEPTION_SET(INSUFFICIENT_MEMORY);
		return;
	}
	memset(
		(char*)dataSet->componentsAvailable,
		0,
		sizeof(bool) * dataSet->componentsList.count);
}

static StatusCode initWithMemory(
	DataSetIpi* dataSet,
	MemoryReader* reader,
	Exception* exception) {
	StatusCode status = SUCCESS;

	// Indicate that the data is in memory and there is no connection to the
	// source data file.
	dataSet->b.b.isInMemory = true;

	// Check that the reader is configured correctly.
	if (reader->current == NULL) {
		return NULL_POINTER;
	}

	// Copy the bytes that form the header from the start of the memory
	// location to the data set data.ptr provided.
	status = readHeaderFromMemory(reader, &dataSet->header);
	if (status != SUCCESS) {
		return status;
	}

	// Check the version.
	status = checkVersion(dataSet);
	if (status != SUCCESS) {
		return status;
	}

	// Create each of the collections.
	const uint32_t stringsCount = dataSet->header.strings.count;
	*(uint32_t*)(&dataSet->header.strings.count) = 0;
	COLLECTION_CREATE_MEMORY(strings)
	*(uint32_t*)(&dataSet->header.strings.count) = stringsCount;

	// Override the header count so that the variable collection can work.
	const uint32_t componentCount = dataSet->header.components.count;
	*(uint32_t*)(&dataSet->header.components.count) = 0;
	COLLECTION_CREATE_MEMORY(components)
	*(uint32_t*)(&dataSet->header.components.count) = componentCount;

	COLLECTION_CREATE_MEMORY(maps)
	COLLECTION_CREATE_MEMORY(properties)
	COLLECTION_CREATE_MEMORY(values)

	const uint32_t profileCount = dataSet->header.profiles.count;
	*(uint32_t*)(&dataSet->header.profiles.count) = 0;
	COLLECTION_CREATE_MEMORY(profiles)
	*(uint32_t*)(&dataSet->header.profiles.count) = profileCount;

	COLLECTION_CREATE_MEMORY(graphs);

	COLLECTION_CREATE_MEMORY(profileGroups);
	COLLECTION_CREATE_MEMORY(propertyTypes);
	COLLECTION_CREATE_MEMORY(profileOffsets);

	dataSet->graphsArray = fiftyoneDegreesIpiGraphCreateFromMemory(
		dataSet->graphs,
		reader,
		exception);

	/* Check that the current pointer equals the last byte */
	if (reader->lastByte != reader->current) {
		return POINTER_OUT_OF_BOUNDS;
	}

	initDataSetPost(dataSet, exception);

	return status;
}

static StatusCode initInMemory(
	DataSetIpi* dataSet,
	Exception* exception) {
	MemoryReader reader;

	// Read the data from the source file into memory using the reader to 
	// store the pointer to the first and last bytes.
	StatusCode status = DataSetInitInMemory(
		&dataSet->b.b,
		&reader);
	if (status != SUCCESS) {
		freeDataSet(dataSet);
		return status;
	}

	// Use the memory reader to initialize the IP Intelligence data set.
	status = initWithMemory(dataSet, &reader, exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		freeDataSet(dataSet);
		return status;
	}
	return status;
}

static void initDataSet(DataSetIpi* dataSet, ConfigIpi** config) {
	// If no config has been provided then use the balanced configuration.
	if (*config == NULL) {
		*config = &IpiBalancedConfig;
	}

	// Reset the data set so that if a partial initialise occurs some memory
	// can freed.
	resetDataSet(dataSet);

	// Copy the configuration into the data set to ensure it's always
	// available in cases where the source configuration gets freed.
	memcpy((void*)&dataSet->config, *config, sizeof(ConfigIpi));
	dataSet->b.b.config = &dataSet->config;
}

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY

static StatusCode readHeaderFromFile(
	FILE* file,
	const DataSetIpiHeader* header) {
	// Read the bytes that make up the dataset header.
	if (fread(
		(void*)header,
		sizeof(DataSetIpiHeader),
		1,
		file) != 1) {
		return CORRUPT_DATA;
	}

	return SUCCESS;
}

static StatusCode readDataSetFromFile(
	DataSetIpi* dataSet,
	FILE* file,
	Exception* exception) {
	StatusCode status = SUCCESS;

	// Copy the bytes that form the header from the start of the memory
	// location to the data set data.ptr provided
	status = readHeaderFromFile(file, &dataSet->header);
	if (status != SUCCESS) {
		return status;
	}

	// Check the version.
	status = checkVersion(dataSet);
	if (status != SUCCESS) {
		return status;
	}

	// Create the strings collection.
	const uint32_t stringsCount = dataSet->header.strings.count;
	*(uint32_t*)(&dataSet->header.strings.count) = 0;
	COLLECTION_CREATE_FILE(strings, fiftyoneDegreesStoredBinaryValueRead);
	*(uint32_t*)(&dataSet->header.strings.count) = stringsCount;

	// Override the header count so that the variable collection can work.
	const uint32_t componentCount = dataSet->header.components.count;
	*(uint32_t*)(&dataSet->header.components.count) = 0;
	COLLECTION_CREATE_FILE(components, fiftyoneDegreesComponentReadFromFile);
	*(uint32_t*)(&dataSet->header.components.count) = componentCount;

	COLLECTION_CREATE_FILE(maps, CollectionReadFileFixed);
	COLLECTION_CREATE_FILE(properties, CollectionReadFileFixed);
	COLLECTION_CREATE_FILE(values, CollectionReadFileFixed);

	const uint32_t profileCount = dataSet->header.profiles.count;
	*(uint32_t*)(&dataSet->header.profiles.count) = 0;
	COLLECTION_CREATE_FILE(profiles, fiftyoneDegreesProfileReadFromFile);
	*(uint32_t*)(&dataSet->header.profiles.count) = profileCount;

	COLLECTION_CREATE_FILE(graphs, CollectionReadFileFixed);

	COLLECTION_CREATE_FILE(profileGroups, CollectionReadFileFixed);
	COLLECTION_CREATE_FILE(propertyTypes, CollectionReadFileFixed);
	COLLECTION_CREATE_FILE(profileOffsets, CollectionReadFileFixed);

	dataSet->graphsArray = fiftyoneDegreesIpiGraphCreateFromFile(
		dataSet->graphs,
		file,
		&dataSet->b.b.filePool,
		// This is not the configuration for the collection of all graphs, but
		// the configuration for each individual graph.
		dataSet->config.graph,
		exception);

	initDataSetPost(dataSet, exception);

	return status;
}

#endif

/**
 * Calculates the highest concurrency value to ensure sufficient file reader
 * handles are generated at initialisation to service the maximum number of
 * concurrent operations.
 * @param config being used for initialisation.
 * @return the highest concurrency value from the configuration, or 1 if no
 * concurrency values are available.
 */
static uint16_t getMaxConcurrency(const ConfigIpi* config) {
	uint16_t concurrency = 1;
	MAX_CONCURRENCY(strings);
	MAX_CONCURRENCY(components);
	MAX_CONCURRENCY(maps);
	MAX_CONCURRENCY(properties);
	MAX_CONCURRENCY(values);
	MAX_CONCURRENCY(profiles);
	MAX_CONCURRENCY(graphs);
	MAX_CONCURRENCY(profileOffsets);
	MAX_CONCURRENCY(propertyTypes);
	MAX_CONCURRENCY(profileGroups);
	MAX_CONCURRENCY(graph);
	return concurrency;
}

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY

static StatusCode initWithFile(DataSetIpi* dataSet, Exception* exception) {
	StatusCode status;
	FileHandle handle;

	// Initialise the file read for the dataset. Create as many readers as
	// there will be concurrent operations.
	status = FilePoolInit(
		&dataSet->b.b.filePool,
		dataSet->b.b.fileName,
		getMaxConcurrency(&dataSet->config),
		exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		return status;
	}

	// Create a new file handle for the read operation. The file handle can't
	// come from the pool of handles because there may only be one available
	// in the pool and it will be needed for some initialisation activities.
	status = FileOpen(dataSet->b.b.fileName, &handle.file);
	if (status != SUCCESS) {
		return status;
	}

	// Read the data set from the source.
	status = readDataSetFromFile(dataSet, handle.file, exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		fclose(handle.file);
		return status;
	}

	// Before closing the file handle, clean up any other temp files which are
	// not in use.
#ifndef __APPLE__
	if (dataSet->config.b.useTempFile == true) {
		FileDeleteUnusedTempFiles(
			dataSet->b.b.masterFileName,
			dataSet->config.b.tempDirs,
			dataSet->config.b.tempDirCount,
			sizeof(DataSetIpiHeader));
	}
#endif
	// Close the file handle.
	fclose(handle.file);

	return status;
}

#endif

static StatusCode initDataSetFromFile(
	void* dataSetBase,
	const void* configBase,
	PropertiesRequired* properties,
	const char* fileName,
	Exception* exception) {
	DataSetIpi* dataSet = (DataSetIpi*)dataSetBase;
	ConfigIpi* config = (ConfigIpi*)configBase;
	StatusCode status = NOT_SET;

	// Common data set initialisation actions.
	initDataSet(dataSet, &config);

	// Initialise the super data set with the filename and configuration
	// provided.
	status = DataSetInitFromFile(
		&dataSet->b.b,
		fileName,
		sizeof(DataSetIpiHeader));
	if (status != SUCCESS) {
		return status;
	}

	// If there is no collection configuration the the entire data file should
	// be loaded into memory. Otherwise use the collection configuration to
	// partially load data into memory and cache the rest.
	if (config->b.allInMemory == true) {
		status = initInMemory(dataSet, exception);
	}
	else {
#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
		status = initWithFile(dataSet, exception);
#else
		status = INVALID_CONFIG;
#endif
	}

	// Return the status code if something has gone wrong.
	if (status != SUCCESS || EXCEPTION_FAILED) {
		// Delete the temp file if one has been created.
		if (config->b.useTempFile == true) {
			FileDelete(dataSet->b.b.fileName);
		}
		return status;
	}

	// Initialise the required properties and headers and check the 
	// initialisation was successful.
	status = initPropertiesAndHeaders(dataSet, properties, exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		// Delete the temp file if one has been created.
		if (config->b.useTempFile == true) {
			FileDelete(dataSet->b.b.fileName);
		}
		return status;
	}

	// Initialise the components available to flag which components have 
	// properties which are to be returned (i.e. available properties).
	status = initComponentsAvailable(dataSet, exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		if (config->b.useTempFile == true) {
			FileDelete(dataSet->b.b.fileName);
		}
		return status;
	}

	// Check there are properties available for retrieval.
	if (dataSet->b.b.available->count == 0) {
		// Delete the temp file if one has been created.
		if (config->b.useTempFile == true) {
			FileDelete(dataSet->b.b.fileName);
		}
		return status;
	}
	return status;
}

fiftyoneDegreesStatusCode fiftyoneDegreesIpiInitManagerFromFile(
	fiftyoneDegreesResourceManager* manager,
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	const char* fileName,
	fiftyoneDegreesException* exception) {

	DataSetIpi* dataSet = (DataSetIpi*)Malloc(sizeof(DataSetIpi));
	if (dataSet == NULL) {
		return INSUFFICIENT_MEMORY;
	}

	StatusCode status = initDataSetFromFile(
		dataSet,
		config,
		properties,
		fileName,
		exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		return status;
	}
	ResourceManagerInit(manager, dataSet, &dataSet->b.b.handle, freeDataSet);
	if (dataSet->b.b.handle == NULL) {
		freeDataSet(dataSet);
		status = INSUFFICIENT_MEMORY;
	}
	return status;
}

size_t fiftyoneDegreesIpiSizeManagerFromFile(
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	const char* fileName,
	fiftyoneDegreesException* exception) {
	size_t allocated;
	ResourceManager manager;
#ifdef _DEBUG 
	StatusCode status;
#endif

	// Set the memory allocation and free methods for tracking.
	MemoryTrackingReset();
	Malloc = MemoryTrackingMalloc;
	MallocAligned = MemoryTrackingMallocAligned;
	Free = MemoryTrackingFree;
	FreeAligned = MemoryTrackingFreeAligned;

	// Initialise the manager with the tracking methods in use to determine
	// the amount of memory that is allocated.
#ifdef _DEBUG 
	status =
#endif
		IpiInitManagerFromFile(
			&manager,
			config,
			properties,
			fileName,
			exception);
#ifdef _DEBUG
	assert(status == SUCCESS);
#endif
	assert(EXCEPTION_OKAY);

	// Free the manager and get the total maximum amount of allocated memory
	// needed for the manager and associated resources.
	ResourceManagerFree(&manager);
	allocated = MemoryTrackingGetMax();

	// Check that all the memory has been freed.
	assert(MemoryTrackingGetAllocated() == 0);

	// Return the malloc and free methods to standard operation.
	Malloc = MemoryStandardMalloc;
	MallocAligned = MemoryStandardMallocAligned;
	Free = MemoryStandardFree;
	FreeAligned = MemoryStandardFreeAligned;
	MemoryTrackingReset();

	return allocated;
}

static StatusCode initDataSetFromMemory(
	void* dataSetBase,
	const void* configBase,
	PropertiesRequired* properties,
	void* memory,
	FileOffset size,
	Exception* exception) {
	StatusCode status = SUCCESS;
	MemoryReader reader;
	DataSetIpi* dataSet = (DataSetIpi*)dataSetBase;
	ConfigIpi* config = (ConfigIpi*)configBase;

	// Common data set initialisation actions.
	initDataSet(dataSet, &config);

	// If memory is to be freed when the data set is freed then record the 
	// pointer to the memory location for future reference.
	if (dataSet->config.b.freeData == true) {
		dataSet->b.b.memoryToFree = memory;
	}

	// Set up the reader.
	reader.startByte = reader.current = (byte*)memory;
	reader.length = size;
	reader.lastByte = reader.current + size;

	// Initialise the data set from the memory reader.
	status = initWithMemory(dataSet, &reader, exception);

	// Return the status code if something has gone wrong.
	if (status != SUCCESS || EXCEPTION_FAILED) {
		return status;
	}

	// Initialise the required properties and headers.
	status = initPropertiesAndHeaders(dataSet, properties, exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		return status;
	}

	// Initialise the components available to flag which components have
	// properties which are to be returned (i.e. available properties).
	status = initComponentsAvailable(dataSet, exception);

	return status;
}

fiftyoneDegreesStatusCode fiftyoneDegreesIpiInitManagerFromMemory(
	fiftyoneDegreesResourceManager* manager,
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	void* memory,
	FileOffset size,
	fiftyoneDegreesException* exception) {
	DataSetIpi* dataSet = (DataSetIpi*)Malloc(sizeof(DataSetIpi));
	if (dataSet == NULL) {
		return INSUFFICIENT_MEMORY;
	}

	StatusCode status = initDataSetFromMemory(
		dataSet,
		config,
		properties,
		memory,
		size,
		exception);
	if (status != SUCCESS || EXCEPTION_FAILED) {
		Free(dataSet);
		return status;
	}
	ResourceManagerInit(manager, dataSet, &dataSet->b.b.handle, freeDataSet);
	if (dataSet->b.b.handle == NULL)
	{
		freeDataSet(dataSet);
		status = INSUFFICIENT_MEMORY;
	}
	return status;
}

size_t fiftyoneDegreesIpiSizeManagerFromMemory(
	fiftyoneDegreesConfigIpi* config,
	fiftyoneDegreesPropertiesRequired* properties,
	void* memory,
	FileOffset size,
	fiftyoneDegreesException* exception) {
	size_t allocated;
	ResourceManager manager;
#ifdef _DEBUG
	StatusCode status;
#endif
	// Set the memory allocation and free methods for tracking.
	MemoryTrackingReset();
	Malloc = MemoryTrackingMalloc;
	MallocAligned = MemoryTrackingMallocAligned;
	Free = MemoryTrackingFree;
	FreeAligned = MemoryTrackingFreeAligned;

	// Ensure that the memory used is not freed with the data set.
	ConfigIpi sizeConfig = *config;
	sizeConfig.b.freeData = false;

	// Initialise the manager with the tracking methods in use to determine
	// the amount of memory that is allocated.
#ifdef _DEBUG
	status =
#endif
		IpiInitManagerFromMemory(
			&manager,
			&sizeConfig,
			properties,
			memory,
			size,
			exception);
#ifdef _DEBUG
	assert(status == SUCCESS);
#endif
	assert(EXCEPTION_OKAY);

	// Free the manager and get the total maximum amount of allocated memory
	// needed for the manager and associated resources.
	ResourceManagerFree(&manager);
	allocated = MemoryTrackingGetMax();

	// Check that all the memory has been freed.
	assert(MemoryTrackingGetAllocated() == 0);

	// Return the malloc and free methods to standard operation.
	Malloc = MemoryStandardMalloc;
	MallocAligned = MemoryStandardMallocAligned;
	Free = MemoryStandardFree;
	FreeAligned = MemoryStandardFreeAligned;
	MemoryTrackingReset();

	return allocated;
}

fiftyoneDegreesDataSetIpi* fiftyoneDegreesDataSetIpiGet(fiftyoneDegreesResourceManager* manager) {
	return (DataSetIpi*)DataSetGet(manager);
}

void fiftyoneDegreesDataSetIpiRelease(fiftyoneDegreesDataSetIpi* dataSet) {
	DataSetRelease(&dataSet->b.b);
}

/**
 * Definition of the reload methods from the data set macro.
 */
FIFTYONE_DEGREES_DATASET_RELOAD(Ipi)


/**
 * IP INTELLIGENCE RESULTS METHODS
 */


/* 
 * Note: WeightedItemList functions are now provided by common-cxx/weightedItem.h
 * The local list functions have been removed in favor of:
 * - WeightedItemListInit() 
 * - WeightedItemListRelease()
 * - WeightedItemListFree()
 * - WeightedItemListExtend()
 * - WeightedItemListAdd()
 */

/**
 * Results methods
 */

fiftyoneDegreesResultsIpi* fiftyoneDegreesResultsIpiCreate(
	fiftyoneDegreesResourceManager* manager) {
	ResultsIpi* results;
	DataSetIpi* dataSet;

	// Increment the inUse counter for the active data set so that we can
	// track any results that are created.
	dataSet = (DataSetIpi*)DataSetGet(manager);

	// Create a new instance of results.
	FIFTYONE_DEGREES_ARRAY_CREATE(
		ResultIpi,
		results,
		dataSet->componentsAvailableCount);

	if (results != NULL) {

		// Initialise the results.
		fiftyoneDegreesResultsInit(&results->b, (void*)(&dataSet->b));

		// Reset the property and values list ready for first use sized for 
		// a single value to be returned.
		WeightedItemListInit(&results->values, 1, FIFTYONE_DEGREES_WEIGHTED_ITEM_LIST_DEFAULT_LOAD_FACTOR);
		DataReset(&results->propertyItem.data);
	}
	else {
		DataSetRelease((DataSetBase *)dataSet);
	}

	return results;
}

static void resultsIpiRelease(ResultsIpi* results) {
	if (results->propertyItem.data.ptr != NULL &&
		results->propertyItem.collection != NULL) {
		COLLECTION_RELEASE(
			results->propertyItem.collection,
			&results->propertyItem);
	}
	WeightedItemListRelease(&results->values);
}

void fiftyoneDegreesResultsIpiFree(fiftyoneDegreesResultsIpi* results) {
	resultsIpiRelease(results);
	WeightedItemListFree(&results->values);
	DataSetRelease((DataSetBase*)results->b.dataSet);
	Free(results);
}

static bool addResultsFromIpAddressNoChecks(
	ResultsIpi* results,
	const unsigned char* ipAddress,
	fiftyoneDegreesIpType type,
	fiftyoneDegreesException* exception) {
	const DataSetIpi * const dataSet = (DataSetIpi*)results->b.dataSet;
	for (uint32_t componentIndex = 0;
		componentIndex < dataSet->componentsList.count;
		componentIndex++) {
		if (!dataSet->componentsAvailable[componentIndex]) {
			continue;
		}
		const Component * const component = COMPONENT(dataSet, componentIndex);
		if (!component) {
			continue;
		}
		ResultIpi * const nextResult = &(results->items[results->count]);
		results->count++;
		resultIpiReset(nextResult);
		// Default IP range offset
		nextResult->graphResult = FIFTYONE_DEGREES_IPI_CG_RESULT_DEFAULT;
		nextResult->graphResult.rawOffset = NULL_PROFILE_OFFSET; // Default IP range offset
		nextResult->targetIpAddress.type = type;
		nextResult->type = type;

		if (type == IP_TYPE_IPV4) {
			// We only get the exact length of ipv4
			memset(nextResult->targetIpAddress.value, 0, IPV6_LENGTH);
			memcpy(nextResult->targetIpAddress.value, ipAddress, IPV4_LENGTH);
		}
		else {
			// We only get the exact length of ipv6
			memcpy(nextResult->targetIpAddress.value, ipAddress, IPV6_LENGTH);
		}

		setResultFromIpAddress(
			nextResult,
			dataSet,
			component->componentId,
			exception);
		if (EXCEPTION_FAILED) {
			return false;
		}
	}
	return true;
}

void fiftyoneDegreesResultsIpiFromIpAddress(
	fiftyoneDegreesResultsIpi* results,
	const unsigned char* ipAddress,
	size_t ipAddressLength,
	fiftyoneDegreesIpType type,
	fiftyoneDegreesException* exception) {

	// Make sure the input is always in the correct format
	if (type == IP_TYPE_INVALID
		|| (type == IP_TYPE_IPV4
			&& ipAddressLength < IPV4_LENGTH)
		|| (type == IP_TYPE_IPV6
			&& ipAddressLength < IPV6_LENGTH)) {
		EXCEPTION_SET(INCORRECT_IP_ADDRESS_FORMAT);
		return;
	}

	// Reset the results data before iterating the evidence.
	results->count = 0;

	addResultsFromIpAddressNoChecks(
		results,
		ipAddress,
		type,
		exception);
}

void fiftyoneDegreesResultsIpiFromIpAddressString(
	fiftyoneDegreesResultsIpi* results,
	const char* ipAddress,
	size_t ipLength,
	fiftyoneDegreesException* exception) {
	IpAddress ip;
	const bool parsed =
		IpAddressParse(ipAddress, ipAddress + ipLength, &ip);
	// Check if the IP address was successfully created
	if (!parsed) {
		EXCEPTION_SET(INCORRECT_IP_ADDRESS_FORMAT);
		return;
	}
	
	// Perform the search on the IP address byte array
	switch(ip.type) {
	case IP_TYPE_IPV4:
		fiftyoneDegreesResultsIpiFromIpAddress(
			results,
			ip.value,
			IPV4_LENGTH,
			IP_TYPE_IPV4,
			exception);
		break;
	case IP_TYPE_IPV6:
		fiftyoneDegreesResultsIpiFromIpAddress(
			results,
			ip.value,
			IPV6_LENGTH,
			IP_TYPE_IPV6,
			exception);
		break;
	case IP_TYPE_INVALID:
	default:
		EXCEPTION_SET(INCORRECT_IP_ADDRESS_FORMAT);
		break;
	}
}

static bool setResultsFromEvidence(
	void* state,
	EvidenceKeyValuePair* pair) {
	const stateWithUniqueHeaderIndex* indexState = (stateWithUniqueHeaderIndex*)state;
	const stateWithException* exceptionState = (stateWithException*)indexState->subState;
	ResultsIpi* results = (ResultsIpi*)exceptionState->state;
	Exception* exception = exceptionState->exception;
	// We should not look further if a 
	// result has already been found
	if (results->count == 0) {
		const DataSetIpi* dataSet = (DataSetIpi*)results->b.dataSet;
		const uint32_t curHeaderIndex = indexState->headerIndex;
		const int headerIndex = HeaderGetIndex(
			dataSet->b.b.uniqueHeaders,
			pair->item.key,
			pair->item.keyLength);
		// Only the current header index should be considered
		if (headerIndex >= 0 && headerIndex == (int)curHeaderIndex) {
			// Get the parsed Value
			const char *ipAddressString = (const char *)pair->parsedValue;
			// Obtain the byte array first
			IpAddress ipAddress;
			const bool parsed =
				fiftyoneDegreesIpAddressParse(ipAddressString, ipAddressString + strlen(ipAddressString), &ipAddress);
			// Check if the IP address was successfully created
			if (!parsed || (ipAddress.type == IP_TYPE_INVALID)) {
				EXCEPTION_SET(INCORRECT_IP_ADDRESS_FORMAT);
				return false;
			}

			// Configure the next result in the array of results.
			addResultsFromIpAddressNoChecks(
				results,
				ipAddress.value,
				ipAddress.type,
				exception);
		}
	}

	return EXCEPTION_OKAY;
}

static void fiftyoneDegreesIterateHeadersWithEvidence(
	ResultsIpi* const results,
	EvidenceKeyValuePairArray* evidence,
	int prefixes,
	stateWithUniqueHeaderIndex *state) {

	const DataSetIpi * const dataSet = (DataSetIpi *)results->b.dataSet;
	const uint32_t headersCount = dataSet->b.b.uniqueHeaders ? dataSet->b.b.uniqueHeaders->count : 0;

	// Each unique header is checked against the evidence
	// in the order that its added to the headers array.
	// The order represents the prioritis of the headers.
	for (uint32_t i = 0;
		i < headersCount && results->count == 0;
		i++) {
		state->headerIndex = i;
		EvidenceIterate(
			evidence,
			prefixes,
			state,
			setResultsFromEvidence);
	}
}

void fiftyoneDegreesResultsIpiFromEvidence(
	fiftyoneDegreesResultsIpi* results,
	fiftyoneDegreesEvidenceKeyValuePairArray* evidence,
	fiftyoneDegreesException* exception) {
	stateWithException subState;
	stateWithUniqueHeaderIndex state;
	subState.state = results;
	subState.exception = exception;
	state.subState = &subState;

	if (evidence != (EvidenceKeyValuePairArray*)NULL) {
		// Reset the results data before iterating the evidence.
		results->count = 0;

		fiftyoneDegreesIterateHeadersWithEvidence(
			results,
			evidence,
			FIFTYONE_DEGREES_EVIDENCE_QUERY,
			&state);
		if (EXCEPTION_FAILED) {
			return;
		}

		// If no results were obtained from the query evidence prefix then use
		// the HTTP headers to populate the results.
		if (results->count == 0) {
			fiftyoneDegreesIterateHeadersWithEvidence(
				results,
				evidence,
				FIFTYONE_DEGREES_EVIDENCE_SERVER,
				&state);
			if (EXCEPTION_FAILED) {
				return;
			}
		}
	}
}

static bool addWeightedValue(
	ResultsIpi* results,
	Item* item,
	uint16_t rawWeighting,
	Exception* exception) {
	Item valueItem;
	WeightedItem weightedItem;
	const DataSetIpi* dataSet = (DataSetIpi*)results->b.dataSet;
	const Value* value = (Value*)item->data.ptr;
	if (value != NULL) {
		if (results->values.count == results->values.capacity) {
			WeightedItemListExtend(
				&results->values,
				results->values.capacity
				* FIFTYONE_DEGREES_WEIGHTED_ITEM_LIST_RESIZE_FACTOR,
				exception);
			if (EXCEPTION_FAILED) {
				COLLECTION_RELEASE(dataSet->values, item);
				return false;
			}
		}
		PropertyValueType const storedValueType = PropertyGetStoredTypeByIndex(
			dataSet->propertyTypes,
			value->propertyIndex,
			exception);
		if (EXCEPTION_OKAY) {
			DataReset(&valueItem.data);
			const uint16_t valueRawWeight = ValueGetWeight(value);
			const uint16_t valueWeight = valueRawWeight ? valueRawWeight : 0xFFFFU;
			if (StoredBinaryValueGet(
				dataSet->strings,
				value->nameOffset,
				storedValueType,
				&valueItem,
				exception) != NULL && EXCEPTION_OKAY) {
				weightedItem.item = valueItem;
				weightedItem.rawWeighting = ((uint32_t)rawWeighting) * (uint32_t)valueWeight;
				WeightedItemListAdd(&results->values, &weightedItem, exception);
			}
		}
	}
	COLLECTION_RELEASE(dataSet->values, item);
	return EXCEPTION_OKAY;
}

static bool addWeightedValueWithState(void* state, Item* item) {
	// The results values are a list of collection items and their weighting.
	// The weighting cannot be passed along with Item as this is the profile
	// standard in common-cxx. Thus the weighting is passed along with the state.
	const stateWithWeighting* weightingState = (stateWithWeighting*)((stateWithException*)state)->state;
	ResultsIpi* results =
		(ResultsIpi*)weightingState->subState;
	Exception* const exception = ((stateWithException*)state)->exception;

	return addWeightedValue(
		results,
		item,
		weightingState->rawWeighting,
		exception);
}

static uint32_t addValuesFromProfile(
	DataSetIpi* dataSet,
	ResultsIpi* results,
	Profile* profile,
	Property* property,
	uint16_t rawWeighting,
	Exception* exception) {
	uint32_t count;

	// Set the state for the callbacks.
	stateWithException state;
	stateWithWeighting weightingState;
	weightingState.subState = results;
	weightingState.rawWeighting = rawWeighting;
	state.state = &weightingState;
	state.exception = exception;

	// Iterate over the values associated with the property adding them
	// to the list of values. Get the number of values available as 
	// this will be used to increase the size of the list should there
	// be insufficient space.
	count = ProfileIterateValuesForProperty(
		dataSet->values,
		profile,
		property,
		&state,
		addWeightedValueWithState,
		exception);
	EXCEPTION_THROW;

	// The count of values should always be lower or the same as the profile
	// count. i.e. there can never be more values counted than there are values
	// against the profile.
	assert(count <= profile->valueCount);

	return count;
}

static uint32_t addValuesFromSingleProfile(
	ResultsIpi* results,
	Property *property,
	uint32_t profileOffset,
	uint16_t rawWeighting,
	Exception* exception) {
	uint32_t count = 0;
	Item profileItem;
	DataSetIpi* dataSet = (DataSetIpi*)results->b.dataSet;

	// Add values from profiles
	Profile *profile = NULL;
	if (profileOffset != NULL_PROFILE_OFFSET) {
		DataReset(&profileItem.data);
		const CollectionKey profileKey = {
			profileOffset,
			CollectionKeyType_Profile,
		};
		profile = (Profile*)dataSet->profiles->get(
			dataSet->profiles,
			&profileKey,
			&profileItem,
			exception);
		// If profile is found
		if (profile != NULL && EXCEPTION_OKAY) {
			count += addValuesFromProfile(
				dataSet,
				results,
				profile,
				property,
				rawWeighting,
				exception);
			COLLECTION_RELEASE(dataSet->profiles, &profileItem);
		}
	}
	return count;
}

static const CollectionKeyType CollectionKeyType_OffsetPercentage = {
	FIFTYONE_DEGREES_COLLECTION_ENTRY_TYPE_OFFSET_PERCENTAGE,
	sizeof(offsetPercentage),
	NULL,
};

static uint32_t addValuesFromProfileGroup(
	ResultsIpi * const results,
	Property * const property,
	const uint32_t profileGroupOffset,
	Exception * const exception) {
	uint32_t count = 0;
	const DataSetIpi * const dataSet = (const DataSetIpi*)results->b.dataSet;

	if (profileGroupOffset == NULL_PROFILE_OFFSET) {
		return 0;
	}
	Item profileGroupItem;
	DataReset(&profileGroupItem.data);

	const Collection * const profileGroups = dataSet->profileGroups;
	for (uint32_t totalWeight = 0, nextOffset = profileGroupOffset;
		(totalWeight < FULL_RAW_WEIGHTING) && EXCEPTION_OKAY;
		++nextOffset) {
		const CollectionKey profileGroupKey = {
			nextOffset,
			&CollectionKeyType_OffsetPercentage,
		};
		const offsetPercentage* const nextWeightedProfileOffset = (const offsetPercentage*)profileGroups->get(
			profileGroups,
			&profileGroupKey,
			&profileGroupItem,
			exception);
		if (!(nextWeightedProfileOffset && EXCEPTION_OKAY)) {
			break;
		}
		totalWeight += nextWeightedProfileOffset->rawWeighting;
		if (totalWeight <= FULL_RAW_WEIGHTING) {
			count += addValuesFromSingleProfile(
				results,
				property,
				nextWeightedProfileOffset->offset,
				nextWeightedProfileOffset->rawWeighting,
				exception);
		} else {
			EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_CORRUPT_DATA);
		}
		COLLECTION_RELEASE(dataSet->profileGroups, &profileGroupItem);
	}
	return count;
}

static uint32_t getProfileOffset(
	Collection * const profileOffsets,
	const uint32_t offsetIndex,
	Exception * const exception) {

	Item item;
	DataReset(&item.data);
	const CollectionKey resultKey = {
		offsetIndex,
		CollectionKeyType_Integer,
	};
	const uint32_t * const resultRef = (uint32_t*)profileOffsets->get(
		profileOffsets,
		&resultKey,
		&item,
		exception);
	if (!(resultRef && EXCEPTION_OKAY)) {
		return 0;
	}
	const uint32_t result = *resultRef;
	COLLECTION_RELEASE(profileOffsets, &item);
	return result;
}

/**
 * Achieves the same as getValuesFromResult, but gets the value from the
 * default value in the property. This is used when there is no value in
 * the profile, but the property is mandatory.
 */
static WeightedItem* getDefaultValue(
	ResultsIpi* results,
	Property* property,
	Exception* exception) {
	const DataSetIpi* const dataSet = (DataSetIpi*)results->b.dataSet;
	bool added = false;
	Item valueItem;

	DataReset(&valueItem.data);

	// Get the value from the value index and call the callback. Do not 
	// free the item as the calling function is responsible for this.
	const CollectionKey valueKey = {
		property->defaultValueIndex,
		CollectionKeyType_Value,
	};
	if (dataSet->values->get(
		dataSet->values,
		&valueKey,
		&valueItem,
		exception) != NULL &&
		EXCEPTION_OKAY) {
		added = addWeightedValue(
			results,
			&valueItem,
			FULL_RAW_WEIGHTING,
			exception);
		COLLECTION_RELEASE(dataSet->values, &valueItem);
	}
	return added ? results->values.items : NULL;
}


static uint32_t addValuesFromResult(
	ResultsIpi* results,
	ResultIpi* result,
	Property* property,
	Exception* exception) {
	uint32_t count = 0;
	const DataSetIpi* const dataSet = (DataSetIpi*)results->b.dataSet;

	if (results->count > 0) {
		if (result->graphResult.rawOffset != NULL_PROFILE_OFFSET) {
			if (!result->graphResult.isGroupOffset) {
				const uint32_t profileOffsetValue = getProfileOffset(
					dataSet->profileOffsets,
					result->graphResult.offset,
					exception);

				if (EXCEPTION_OKAY) {
					count += addValuesFromSingleProfile(
						results,
						property,
						profileOffsetValue,
						FULL_RAW_WEIGHTING,
						exception);
				}
			} else {
				count += addValuesFromProfileGroup(
					results,
					property,
					result->graphResult.offset,
					exception);
			}
		}
	}
	return count;
}

static WeightedItem* getValuesFromResult(
	ResultsIpi* results,
	ResultIpi* result,
	Property* property,
	Exception* exception) {
	// There is a profile available for the property requested. 
	// Use this to add the values to the results.
	addValuesFromResult(results, result, property, exception);

	// Return the first value in the list of items.
	return results->values.items;
}

const fiftyoneDegreesWeightedItem* fiftyoneDegreesResultsIpiGetValues(
	fiftyoneDegreesResultsIpi* const results,
	int const requiredPropertyIndex,
	fiftyoneDegreesException* const exception) {
	Property* property;
	DataSetIpi* dataSet;
	const WeightedItem* firstValue = NULL;

	// Ensure any previous uses of the results to get values are released.
	resultsIpiRelease(results);

	dataSet = (DataSetIpi*)results->b.dataSet;

	// Work out the property index from the required property index.
	const uint32_t propertyIndex = PropertiesGetPropertyIndexFromRequiredIndex(
		dataSet->b.b.available,
		requiredPropertyIndex);

	if (propertyIndex >= 0) {
		// Set the property that will be available in the results structure. 
		// This may also be needed to work out which of a selection of results 
		// are used to obtain the values.
		property = PropertyGet(
			dataSet->properties,
			propertyIndex,
			&results->propertyItem,
			exception);

		if (property != NULL && EXCEPTION_OKAY) {
			// Ensure there is a collection available to the property item so
			// that it can be freed when the results are freed.
			if (results->propertyItem.collection == NULL) {
				results->propertyItem.collection = dataSet->properties;
			}

			// There will only be one result
			for (uint32_t i = 0; i < results->count && EXCEPTION_OKAY; i++) {
				firstValue = getValuesFromResult(
					results,
					&results->items[i],
					property,
					exception);
			}

			if (results->values.count == 0 &&
				property->defaultValueIndex != UINT32_MAX &&
				property->isMandatory) {
				// There are no values, but the default value from the property
				// should be used, as there is a default value, and the property
				// is marked mandatory.
				firstValue = getDefaultValue(
					results,
					property,
					exception);
			}
		}
	}

	if (firstValue == NULL) {
		// There are no values for the property requested. Reset the values 
		// list to zero count.
		WeightedItemListRelease(&results->values);
	}
	return firstValue;
}

static bool visitProfilePropertyValue(
	void *state,
	fiftyoneDegreesCollectionItem *item) {
#	ifdef _MSC_VER
	UNREFERENCED_PARAMETER(item);
#	endif

	*((bool *)state) = true; // found
	return false; // break
}

static bool profileHasValidPropertyValue(
	const DataSetIpi * const dataSet,
	const uint32_t profileOffset,
	Property * const property,
	Exception * const exception) {
	Item profileItem;
	Profile *profile = NULL;
	bool valueFound = false;

	if (profileOffset != NULL_PROFILE_OFFSET) {
		DataReset(&profileItem.data);
		const CollectionKey profileKey = {
			profileOffset,
			CollectionKeyType_Profile,
		};
		profile = (Profile*)dataSet->profiles->get(
			dataSet->profiles,
			&profileKey,
			&profileItem,
			exception);
		// If profile is found
		if (profile != NULL && EXCEPTION_OKAY) {
			ProfileIterateValuesForProperty(
				dataSet->values,
				profile,
				property,
				&valueFound,
				visitProfilePropertyValue,
				exception);
			COLLECTION_RELEASE(dataSet->profiles, &profileItem);
		}
	}
	return valueFound;
}

static bool resultGetHasValidPropertyValueOffset(
	fiftyoneDegreesResultsIpi* const results,
	const fiftyoneDegreesResultIpi* const result,
	const int requiredPropertyIndex,
	fiftyoneDegreesException* const exception) {
	bool hasValidOffset = false;
	Item item;
	DataReset(&item.data);
	const DataSetIpi * const dataSet = (DataSetIpi*)results->b.dataSet;

	// Work out the property index from the required property index.
	const int32_t propertyIndex = PropertiesGetPropertyIndexFromRequiredIndex(
		dataSet->b.b.available,
		requiredPropertyIndex);

	if (propertyIndex >= 0) {
		// Set the property that will be available in the results structure.
		// This may also be needed to work out which of a selection of results
		// are used to obtain the values.
		Property * const property = PropertyGet(
			dataSet->properties,
			propertyIndex,
			&results->propertyItem,
			exception);

		const char * const propertyName = STRING( // name is string
			PropertiesGetNameFromRequiredIndex(
				dataSet->b.b.available,
				requiredPropertyIndex));
		if (propertyName != NULL && EXCEPTION_OKAY) {
			// We will only execute this step if successfully obtained the
			// profile groups offset from the previous step
			if (result->graphResult.rawOffset != NULL_PROFILE_OFFSET) {
				if (!result->graphResult.isGroupOffset) {
					const uint32_t profileOffsetValue = getProfileOffset(
						dataSet->profileOffsets,
						result->graphResult.offset,
						exception);
					if (EXCEPTION_OKAY) {
						hasValidOffset = profileHasValidPropertyValue(
							dataSet,
							profileOffsetValue,
							property,
							exception);
					}
				} else {
					const Collection * const profileGroups = dataSet->profileGroups;
					for (uint32_t totalWeight = 0,
						nextOffset = result->graphResult.offset;
						(!hasValidOffset) && (totalWeight < FULL_RAW_WEIGHTING) && EXCEPTION_OKAY;
						++nextOffset) {
						const CollectionKey profileGroupKey = {
							nextOffset,
							&CollectionKeyType_OffsetPercentage,
						};
						const offsetPercentage* const nextWeightedProfileOffset = (const offsetPercentage*)profileGroups->get(
							profileGroups,
							&profileGroupKey,
							&item,
							exception);
						if (!(nextWeightedProfileOffset && EXCEPTION_OKAY)) {
							break;
						}
						totalWeight += nextWeightedProfileOffset->rawWeighting;
						if (totalWeight <= FULL_RAW_WEIGHTING) {
							hasValidOffset = profileHasValidPropertyValue(
								dataSet, nextWeightedProfileOffset->offset, property, exception);
						} else {
							EXCEPTION_SET(FIFTYONE_DEGREES_STATUS_CORRUPT_DATA);
						}
						COLLECTION_RELEASE(profileGroups, &item);
					}
				}
			}
		}
	}
	return hasValidOffset;
}

bool fiftyoneDegreesResultsIpiGetHasValues(
	fiftyoneDegreesResultsIpi* results,
	int requiredPropertyIndex,
	fiftyoneDegreesException* exception) {
	const DataSetIpi *dataSet = (DataSetIpi*)results->b.dataSet;
	// Ensure any previous uses of the results to get values are released.
	resultsIpiRelease(results);

	if (requiredPropertyIndex < 0 ||
		requiredPropertyIndex >= (int)dataSet->b.b.available->count) {
		// The property index is not valid.
		return false;
	}

	if (results->count == 0)
		// There is no result
		return false;

	// There will only be one result
	for (uint32_t i = 0; i < results->count; i++) {
		const bool hasValidOffset = resultGetHasValidPropertyValueOffset(
			results,
			&results->items[i],
			requiredPropertyIndex,
			exception);
		if (EXCEPTION_FAILED) {
			return false;
		}
		if (hasValidOffset) {
			return true;
		}
	}
	const uint32_t propertyIndex = PropertiesGetPropertyIndexFromRequiredIndex(
		dataSet->b.b.available,
		requiredPropertyIndex);

	if (propertyIndex >= 0) {
		Property *property = PropertyGet(
			dataSet->properties,
			propertyIndex,
			&results->propertyItem,
			exception);
		if (property != NULL && EXCEPTION_OKAY) {
			if (property->defaultValueIndex != UINT32_MAX &&
				property->isMandatory) {
				// Although there is no values, the property is mandatory,
				// and there is a default value which will be used. So
				// return true.
				return true;
			}
		}
	}
	return false;
}

fiftyoneDegreesResultsNoValueReason fiftyoneDegreesResultsIpiGetNoValueReason(
	fiftyoneDegreesResultsIpi* results,
	int requiredPropertyIndex,
	fiftyoneDegreesException* exception) {
	const DataSetIpi *dataSet = (DataSetIpi*)results->b.dataSet;
	// Ensure any previous uses of the results to get values are released.
	resultsIpiRelease(results);

	if (requiredPropertyIndex < 0 ||
		requiredPropertyIndex >= (int)dataSet->b.b.available->count) {
		return FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_INVALID_PROPERTY;
	}

	if (results->count == 0) {
		return FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NO_RESULTS;
	}

	// There will only be one result
	for (uint32_t i = 0; i < results->count; i++) {
		const bool hasValidOffset = resultGetHasValidPropertyValueOffset(
			results,
			&results->items[i],
			requiredPropertyIndex,
			exception);
		if (EXCEPTION_FAILED) {
			return false;
		}
		if (hasValidOffset) {
			return FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_UNKNOWN;
		}
	}
	if (EXCEPTION_OKAY) {
		return FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NULL_PROFILE;
	}

	return FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_UNKNOWN;
}

const char* fiftyoneDegreesResultsIpiGetNoValueReasonMessage(
	fiftyoneDegreesResultsNoValueReason reason) {
	switch (reason) {
	case FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NO_RESULTS:
		return "The results are empty. This is probably because we don't "
			"have this data in our database.";
	case FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_NULL_PROFILE:
		return "The results contained a null profile for the component which "
			"the required property belongs to.";
	case FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_INVALID_PROPERTY:
		return "The requested property does not exist, or is not a required property";
	case FIFTYONE_DEGREES_RESULTS_NO_VALUE_REASON_UNKNOWN:
	default:
		return "The reason for missing values is unknown.";
	}
}

static void pushValues(
	const WeightedItem * const weightedItem,
	const uint32_t count,
	StringBuilder * const builder,
	const char * const separator,
	PropertyValueType storedValueType,
	const uint8_t decimalPlaces,
	Exception * const exception) {

	const size_t sepLen = strlen(separator);

	// Loop through the values adding them to the string buffer.
	for (uint32_t i = 0; i < count;  i++) {
		// Append the separator
		if (i) {
			StringBuilderAddChars(builder, separator, sepLen);
		}

		// Add the opening quote
		StringBuilderAddChar(builder, '"');

		// Get the string for the value index.
		const StoredBinaryValue * const binaryValue =
			(const StoredBinaryValue*)weightedItem[i].item.data.ptr;

		// Add the string to the output buffer recording the number
		// of characters added.
		StringBuilderAddStringValue(
			builder,
			binaryValue,
			storedValueType,
			decimalPlaces,
			exception);

		// Add the closing quote
		StringBuilderAddChar(builder, '"');
		StringBuilderAddChar(builder, ':');
		StringBuilderAddDouble(
			builder,
			(double)weightedItem[i].rawWeighting / (double)FIFTYONE_DEGREES_WEIGHTED_ITEM_MAX_WEIGHT,
			decimalPlaces);
	}
}

static void fiftyoneDegreesResultsIpiGetValuesStringInternal(
	fiftyoneDegreesResultsIpi* results,
	int requiredPropertyIndex,
	StringBuilder * const builder,
	const char* separator,
	fiftyoneDegreesException* exception) {
	const WeightedItem *weightedItem;
	Item propertyItem;
	Property *property;
	const DataSetIpi *dataSet = (DataSetIpi *)results->b.dataSet;

	const int propertyIndex = PropertiesGetPropertyIndexFromRequiredIndex(
		dataSet->b.b.available,
		requiredPropertyIndex);

	if (propertyIndex >= 0) {
		PropertyValueType const storedValueType = PropertyGetStoredTypeByIndex(
			dataSet->propertyTypes,
			propertyIndex,
			exception);
		if (EXCEPTION_FAILED) {
			return;
		}
		DataReset(&propertyItem.data);
		const CollectionKey propertyKey = {
			propertyIndex,
			CollectionKeyType_Property,
		};
		property = (Property*)dataSet->properties->get(
			dataSet->properties,
			&propertyKey,
			&propertyItem,
			exception);
		if (property != NULL && EXCEPTION_OKAY) {
			if (requiredPropertyIndex >= 0) {
				weightedItem = fiftyoneDegreesResultsIpiGetValues(
					results,
					requiredPropertyIndex,
					exception);
				if (weightedItem != NULL && EXCEPTION_OKAY) {
					pushValues(
						weightedItem,
						results->values.count,
						builder,
						separator,
						storedValueType,
						DefaultWktDecimalPlaces,
						exception);
				}
			}
			COLLECTION_RELEASE(dataSet->properties, &propertyItem);
		}
	}
}

void fiftyoneDegreesResultsIpiAddValuesString(
	fiftyoneDegreesResultsIpi* results,
	const char* propertyName,
	fiftyoneDegreesStringBuilder *builder,
	const char* separator,
	fiftyoneDegreesException* exception) {
	const DataSetIpi * const dataSet = (DataSetIpi *)results->b.dataSet;
	const int requiredPropertyIndex = PropertiesGetRequiredPropertyIndexFromName(
		dataSet->b.b.available,
		propertyName);

	if (requiredPropertyIndex >= 0) {
		fiftyoneDegreesResultsIpiGetValuesStringInternal(
			results,
			requiredPropertyIndex,
			builder,
			separator,
			exception);
	}
}

size_t fiftyoneDegreesResultsIpiGetValuesString(
	fiftyoneDegreesResultsIpi* results,
	const char* propertyName,
	char* buffer,
	size_t bufferLength,
	const char* separator,
	fiftyoneDegreesException* exception) {

	StringBuilder builder = { buffer, bufferLength };
	StringBuilderInit(&builder);

	fiftyoneDegreesResultsIpiAddValuesString(
		results,
		propertyName,
		&builder,
		separator,
		exception);

	StringBuilderComplete(&builder);

	return builder.added;
}

size_t fiftyoneDegreesResultsIpiGetValuesStringByRequiredPropertyIndex(
	fiftyoneDegreesResultsIpi* results,
	const int requiredPropertyIndex,
	char* buffer,
	size_t bufferLength,
	const char* separator,
	fiftyoneDegreesException* exception) {

	StringBuilder builder = { buffer, bufferLength };
	StringBuilderInit(&builder);

	fiftyoneDegreesResultsIpiGetValuesStringInternal(
		results,
		requiredPropertyIndex,
		&builder,
		separator,
		exception);

	StringBuilderComplete(&builder);

	return builder.added;
}

/*
 * Supporting Macros to printout the NetworkId
 */
#define PRINT_PROFILE_SEP(d,b,s,f) printProfileSeparator(&d, (b - d) + s, f)
#define PRINT_PROFILE_ID(d,b,s,f,v,p) printProfileId(&d, (b - d) + s, f, v, p)
#define PRINT_NULL_PROFILE_ID(d,b,s,p) PRINT_PROFILE_ID(d, b, s, "%i:%f", 0, p)

size_t fiftyoneDegreesIpiGetIpAddressAsString(
	const fiftyoneDegreesCollectionItem * const item,
	const fiftyoneDegreesIpType type,
	char * const buffer,
	const uint32_t bufferLength,
	fiftyoneDegreesException * const exception) {

	StringBuilder builder = { buffer, bufferLength };
	StringBuilderInit(&builder);

	StringBuilderAddIpAddress(
		&builder,
		(const VarLengthByteArray *)item->data.ptr,
		type,
		exception);

	return builder.added;
}

uint32_t fiftyoneDegreesIpiIterateProfilesForPropertyAndValue(
	fiftyoneDegreesResourceManager* manager,
	const char* propertyName,
	const char* valueName,
	void* state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException* exception) {
	uint32_t count = 0;
	DataSetIpi* dataSet = DataSetIpiGet(manager);
	count = ProfileIterateProfilesForPropertyWithTypeAndValueAndOffsetExtractor(
		dataSet->strings,
		dataSet->properties,
		dataSet->propertyTypes,
		dataSet->values,
		dataSet->profiles,
		dataSet->profileOffsets,
		ProfileOffsetAsPureOffset,
		propertyName,
		valueName,
		state,
		callback,
		exception);
	DataSetIpiRelease(dataSet);
	return count;
}
