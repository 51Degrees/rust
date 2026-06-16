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

#ifndef FIFTYONE_DEGREES_INDICES_H_INCLUDED
#define FIFTYONE_DEGREES_INDICES_H_INCLUDED

 /**
  * @ingroup FiftyOneDegreesCommon
  * @defgroup FiftyOneDegreesIndices Indices
  *
  * A look up structure for profile and property index to the first value 
  * associated with the property and profile.
  *
  * ## Introduction
  * 
  * Data sets relate profiles to the values associated with them. Values are
  * associated with properties. The values associated with a profile are 
  * ordered in ascending order of property. Therefore when a request is made to
  * obtain the value for a property and profile the values needed to be 
  * searched using a binary search to find a value related to the property. 
  * Then the list of prior values is checked until the first value for the 
  * property is found.
  * 
  * The indices methods provide common functionality to create a structure
  * that directly relates profile ids and required property indexes to the 
  * first value index thus increasing the efficiency of retrieving values.
  * 
  * It is expected these methods will be used during data set initialization.
  * 
  * ## Structure
  * 
  * A sparse array of profile ids and required property indexes is used. Whilst
  * this consumes more linear memory than a binary tree or other structure it 
  * is extremely fast to retrieve values from. As the difference between the 
  * lowest and highest profile id is relatively small the memory associated 
  * with absent profile ids is considered justifiable considering the 
  * performance benefit. A further optimization is to use the required property
  * index rather than the index of all possible properties contained in the 
  * data set. In most use cases the caller only requires a sub set of 
  * properties to be available for retrieval.
  * 
  * ## Create
  * 
  * fiftyoneDegreesIndicesPropertyProfileCreate should be called once the data
  * set is initialized with the required data structures. Memory is allocated
  * by the method and a pointer to the index data structure is returned. The
  * caller is not expected to use the returned data structure directly.
  * 
  * Some working memory is allocated during the indexing process. Therefore 
  * this method must be called before a freeze on allocating new memory is
  * required.
  * 
  * ## Free
  * 
  * fiftyoneDegreesIndicesPropertyProfileFree is used to free the memory used
  * by the index returned from Create. Must be called during the freeing of the
  * related data set.
  * 
  * ## Lookup
  * 
  * fiftyoneDegreesIndicesPropertyProfileLookup is used to return the index in
  * the values associated with the profile for the profile id and the required
  * property index.
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
#include "array.h"
#include "data.h"
#include "exceptions.h"
#include "collection.h"
#include "property.h"
#include "properties.h"
#include "common.h"

/**
 * Maps the profile index and the property index to the first value index of 
 * the profile for the property. Is an array of uint32_t with entries equal to 
 * the number of properties multiplied by the difference between the lowest and
 * highest profile id.
 */
typedef struct fiftyone_degrees_index_property_profile{
	uint32_t* valueIndexes; // array of value indexes
	uint32_t availablePropertyCount; // number of available properties
	uint32_t minProfileId; // minimum profile id
	uint32_t maxProfileId; // maximum profile id
	uint32_t profileCount; // total number of profiles
	uint32_t size; // number elements in the valueIndexes array
	uint32_t filled; // number of elements with values
} fiftyoneDegreesIndicesPropertyProfile;

/**
 * Create an index for the profiles, available properties, and values provided 
 * such that given the index to a property and profile the index of the first 
 * value can be returned by calling fiftyoneDegreesIndicesPropertyProfileLookup.
 * @param profiles collection of variable sized profiles to be indexed
 * @param profileOffsets collection of fixed offsets to profiles to be indexed
 * @param available properties provided by the caller
 * @param values collection to be indexed
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the index memory structure
 */
EXTERNAL fiftyoneDegreesIndicesPropertyProfile*
fiftyoneDegreesIndicesPropertyProfileCreate(
	fiftyoneDegreesCollection* profiles,
	fiftyoneDegreesCollection* profileOffsets,
	fiftyoneDegreesPropertiesAvailable* available,
	fiftyoneDegreesCollection* values,
	fiftyoneDegreesException* exception);

/**
 * Frees an index previously created by 
 * fiftyoneDegreesIndicesPropertyProfileCreate.
 * @param index to be freed
 */
EXTERNAL void fiftyoneDegreesIndicesPropertyProfileFree(
	fiftyoneDegreesIndicesPropertyProfile* index);

/**
 * For a given profile id and available property index returns the first value 
 * index, or null if a first index can not be determined from the index. The
 * indexes relate to the collections for profiles, properties, and values 
 * provided to the fiftyoneDegreesIndicesPropertyProfileCreate method when the 
 * index was created. The availablePropertyIndex is not the index of all 
 * possible properties, but the index of the ones the data set was created 
 * expecting to return.
 * @param index from fiftyoneDegreesIndicesPropertyProfileCreate to use
 * @param profileId the values need to relate to
 * @param availablePropertyIndex in the list of required properties
 * @return the index in the list of values for the profile for the first value 
 * associated with the property
 */
EXTERNAL uint32_t fiftyoneDegreesIndicesPropertyProfileLookup(
	fiftyoneDegreesIndicesPropertyProfile* index,
	uint32_t profileId,
	uint32_t availablePropertyIndex);

/**
 * @}
 */

#endif