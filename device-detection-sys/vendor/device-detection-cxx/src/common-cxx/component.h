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

#ifndef FIFTYONE_DEGREES_COMPONENT_H_INCLUDED
#define FIFTYONE_DEGREES_COMPONENT_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesComponent Component
 *
 * Component of a data set.
 *
 * ## Introduction
 *
 * A Component is stored in a components collection and contains the meta data
 * for a specific component in a data set. Each component relates to a set of
 * properties which a profile relating to the component will hold the values
 * for.
 *
 * ## Get
 *
 * As there are generally only a small number of components in a data set, they
 * can be accessed via a list structure instead of fetching from the base
 * collection. This ensures that a reference the collection item is always held
 * open and can be used at any time.
 *
 * @{
 */

#include <stdint.h>
#ifdef _MSC_VER
#pragma warning (push)
#pragma warning (disable: 5105) 
#include <windows.h>
#pragma warning (default: 5105) 
#pragma warning (pop)
#endif
#include "data.h"
#include "exceptions.h"
#include "collection.h"
#include "list.h"
#include "string.h"
#include "common.h"
#include "headers.h"

/**
 * Key value pair contained in each component. This can point to anything. For
 * example, in the Hash device detection API, the key is the unique id of an
 * HTTP header, and the value is the index of the set of root nodes to use.
 */
typedef struct fiftyoneDegrees_component_keyvaluepair_t {
	uint32_t key; /**< Integer key */
	uint32_t value; /**< Integer value */
} fiftyoneDegreesComponentKeyValuePair;

/**
 * A component of a data set. For example a hardware component contains
 * profiles relating to the hardware properties of a device.
 */
#pragma pack(push, 1)
typedef struct fiftyoneDegrees_component_t {
	const byte componentId; /**< The unique Id of the component. */
	const int32_t nameOffset; /**< Offset in the strings data structure to the
	                               name */
	const int32_t defaultProfileOffset; /**< Offset in the profiles data
	                                         structure to the default profile */
	const uint16_t keyValuesCount; /**< The number of key value pairs at
								   firstKeyValuePair */
	const fiftyoneDegreesComponentKeyValuePair firstKeyValuePair; /**< The
																  first key
																  value pair */
} fiftyoneDegreesComponent;
#pragma pack(pop)

/**
 * Gets size of Component with trailing key-value pair.
 * @param initial pointer to component "head"
 * @return full (with tail) struct size
 */
EXTERNAL uint32_t fiftyoneDegreesComponentGetFinalSize(
	const void *initial,
	fiftyoneDegreesException *exception);
/**
 * Returns the string name of the component using the item provided. The
 * collection item must be released when the caller is finished with the
 * string.
 * @param component structure for the name required.
 * @param stringsCollection collection of strings retrieved by offsets.
 * @param item used to store the resulting string in.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return a pointer to a string in the collection item data structure.
 */
EXTERNAL const fiftyoneDegreesString* fiftyoneDegreesComponentGetName(
	fiftyoneDegreesCollection *stringsCollection,
	fiftyoneDegreesComponent *component,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Get a pointer to the key value pair at the specified index within the
 * component's key value pairs list.
 * This pointer does not need to be freed by the caller.
 * @param component to get the pair from
 * @param index of the pair within the component
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to a key value pair
 */
const fiftyoneDegreesComponentKeyValuePair* 
fiftyoneDegreesComponentGetKeyValuePair(
	fiftyoneDegreesComponent *component,
	uint16_t index,
	fiftyoneDegreesException *exception);

/**
 * Initialises the list of components. This holds a reference to the collection
 * items so that collection get methods do not need to be called repeatedly.
 * @param components collection containing the components to add to the list
 * @param list to add the components to
 * @param count number of components in the collection
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 */
void fiftyoneDegreesComponentInitList(
	fiftyoneDegreesCollection *components,
	fiftyoneDegreesList *list,
	uint32_t count,
	fiftyoneDegreesException *exception);

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY

/**
 * Read a component from the file collection provided and store in the data
 * pointer. This method is used when creating a collection from file.
 * @param file collection to read from
 * @param key of the component in the collection
 * @param data to store the resulting component in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the component allocated within the data structure
 */
EXTERNAL void* fiftyoneDegreesComponentReadFromFile(
	const fiftyoneDegreesCollectionFile *file,
	const fiftyoneDegreesCollectionKey *key,
	fiftyoneDegreesData *data,
	fiftyoneDegreesException *exception);

#endif

/**
 * Get the default profile id for the component provided.
 * @param profiles collection containing the profiles from the same data set as
 * the component
 * @param component to get the default profile id for
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 */
EXTERNAL uint32_t fiftyoneDegreesComponentGetDefaultProfileId(
	fiftyoneDegreesCollection *profiles,
	fiftyoneDegreesComponent *component,
	fiftyoneDegreesException *exception);

/**
 * Where the component's key value pairs relate to headers creates an array of
 * pointers to the relevant headers in the same order as the key value pairs.
 * @param component to get the headers for
 * @param headers array to find the corresponding header in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 */
EXTERNAL fiftyoneDegreesHeaderPtrs* fiftyoneDegreesComponentGetHeaders(
	fiftyoneDegreesComponent* component,
	fiftyoneDegreesHeaders* headers,
	fiftyoneDegreesException* exception);

/**
 * @}
 */

#endif