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

#ifndef FIFTYONE_DEGREES_PROFILE_H_INCLUDED
#define FIFTYONE_DEGREES_PROFILE_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesProfile Profile
 *
 * Profile containing a unique set of values for the properties of a single
 * component.
 *
 * ## Introduction
 *
 * A Profile is stored in a profiles collection and contains the meta data for
 * a specific profile for a component in a data set. Each profile contains
 * a unique set of values for a single component.
 *
 * ## Get
 *
 * A Profile can be fetched from a profiles collection in one of two ways:
 *
 * **By Index** : The #fiftyoneDegreesProfileGetByIndex method return the
 * profile at a specified index. This provides a way to access a profile at a
 * known index, or iterate over all values.
 *
 * **By Id** : If the index of a profile is not known, then the profile can be
 * fetched using the #fiftyoneDegreesProfileGetByProfileId method to find the
 * profile in a profiles collection.
 *
 * ## Iterate
 *
 * A profiles collection can be iterated in two ways:
 *
 * **Values** : The #fiftyoneDegreesProfileIterateValuesForProperty method can
 * be used to iterate over all the values which a profile contains for a
 * specified property.
 *
 * **Profiles** : The #fiftyoneDegreesProfileIterateProfilesForPropertyAndValue
 * method can be used to iterate over all the profiles in a profiles collection
 * which contain a specified property and value pairing.
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
#include "string.h"
#include "component.h"
#include "property.h"
#include "value.h"
#include "common.h"
#include "indices.h"

typedef
#ifdef FIFTYONE_DEGREES_REDUCED_FILE
// In a reduced size data file, there is no profile id, and the value count
// is written as 2 bytes.
uint16_t
#else
// Full size data file contains a profile id, and 32 bit value count.
uint32_t
#endif
fiftyoneDegreesProfileValuesCountType; /**< Type for fiftyoneDegreesProfile::valueCount */

/**
 * Encapsulates a profile stored within a data set. A profile pertains to a
 * specific set of values for the properties relating to a single component.
 */
#pragma pack(push, 1)
typedef struct fiftyoneDegrees_profile_t {
	const byte componentIndex; /**< The index of the component the profile
	                               relates to */
#ifndef FIFTYONE_DEGREES_REDUCED_FILE
	// In a reduced size data file, there is no profile id.
	// Full size data file contains a profile id.
	const uint32_t profileId; /**< Unique id of the profile */
#endif
	const fiftyoneDegreesProfileValuesCountType valueCount; /**< The number of values within the profile */
} fiftyoneDegreesProfile;
#pragma pack(pop)

/**
 * Structure containing the unique id of the profile, and the offset needed to
 * retrieve the actual profile structure from a profiles collection. This takes
 * up less space than the full profile and are of a fixed size, so can be
 * passed around and iterated over more easily.
 */
#pragma pack(push, 4)
typedef struct fiftyoneDegrees_profile_offset_t {
#ifndef FIFTYONE_DEGREES_REDUCED_FILE
	const uint32_t profileId; /**< The unique Id of the profile */
#endif
	const uint32_t offset; /**< Offset to the profile in the profiles structure */
} fiftyoneDegreesProfileOffset;
#pragma pack(pop)

/**
 * Function that extracts "pure" profile offset
 * from a value inside `profileOffsets` collection
 * @param rawProfileOffset a "raw" value retrieved from `profileOffsets`
 * @return Offset to the profile in the profiles structure
 */
typedef uint32_t (*fiftyoneDegreesProfileOffsetValueExtractor)(const void *rawProfileOffset);

/**
 * Function that extracts "pure" profile offset
 * from a fiftyoneDegreesProfileOffset.
 * @param rawProfileOffset a "raw" ProfileOffset retrieved from `profileOffsets`
 * @return Offset to the profile in the profiles structure
 */
uint32_t fiftyoneDegreesProfileOffsetToPureOffset(const void *rawProfileOffset);

/**
 * Function that extracts "pure" profile offset
 * from a value (that starts with a "pure" profile offset)
 * inside `profileOffsets` collection
 * @param rawProfileOffset a "raw" value retrieved from `profileOffsets`
 * @return Offset to the profile in the profiles structure
 */
uint32_t fiftyoneDegreesProfileOffsetAsPureOffset(const void *rawProfileOffset);

/**
 * Definition of a callback function which is passed an item of a type 
 * determined by the iteration method.
 * @param state pointer to data needed by the method
 * @param item to store each profile in while iterating
 * @return true if the iteration should continue, otherwise false to stop
 */
typedef bool(*fiftyoneDegreesProfileIterateMethod)(
	void *state,
	fiftyoneDegreesCollectionItem *item);

/**
 * Definition of a callback function which is passed the next values index for
 * the profile.
 * @param state pointer to data needed by the method
 * @param valueIndex for the next value
 * @return true if the iteration should continue, otherwise false to stop
 */
typedef bool(*fiftyoneDegreesProfileIterateValueIndexesMethod)(
	void* state,
	uint32_t valueIndex);

/**
 * Gets size of Profile with trailing values.
 * @param initial pointer to profile "head"
 * @return full (with tail) struct size
 */
#ifndef FIFTYONE_DEGREES_MEMORY_ONLY
EXTERNAL uint32_t fiftyoneDegreesProfileGetFinalSize(
	const void *initial,
	fiftyoneDegreesException * const exception);
#else
#define fiftyoneDegreesProfileGetFinalSize NULL
#endif

/**
 * Gets the profile associated with the profileId or NULL if there is no
 * corresponding profile.
 * @param profileOffsets collection containing the profile offsets (with profile ID)
 * @param profiles collection containing the profiles referenced by the profile
 * offsets
 * @param profileId the unique id of the profile to fetch
 * @param item to set as the handle to the profile returned
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the profile or NULL
 */
EXTERNAL fiftyoneDegreesProfile* fiftyoneDegreesProfileGetByProfileId(
	fiftyoneDegreesCollection *profileOffsets,
	fiftyoneDegreesCollection *profiles,
	uint32_t profileId,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Gets a pointer to the profile at the index provided. The index refers to the
 * index in the profile offsets collection as this contains fixed size entities
 * which can be quickly looked up. The variable sized profile is then returned
 * from that.
 * @param profileOffsets collection containing the profile offsets
 * @param profiles collection containing the profiles referenced by the profile
 * offsets
 * @param index of the profile to return
 * @param item to set as the handle to the profile returned
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the profile at the index
 */
EXTERNAL fiftyoneDegreesProfile* fiftyoneDegreesProfileGetByIndex(
	fiftyoneDegreesCollection *profileOffsets,
	fiftyoneDegreesCollection *profiles,
	uint32_t index,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

#ifndef FIFTYONE_DEGREES_MEMORY_ONLY

/**
 * Read a profile from the file collection provided and store in the data
 * pointer. This method is used when creating a collection from file.
 * @param file collection to read from
 * @param key of the profile in the collection
 * @param data to store the resulting profile in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the profile allocated within the data structure
 */
EXTERNAL void* fiftyoneDegreesProfileReadFromFile(
	const fiftyoneDegreesCollectionFile *file,
	const fiftyoneDegreesCollectionKey *key,
	fiftyoneDegreesData *data,
	fiftyoneDegreesException *exception);
#endif

/**
 * Iterate over all values contained in the profile which relate to the
 * specified property, calling the callback method for each.
 * @param values collection containing all values
 * @param profile pointer to the profile to iterate the values of
 * @param property which the values must relate to
 * @param state pointer containing data needed for the callback method
 * @param callback method to be called for each value
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number of matching values which have been iterated
 */
EXTERNAL uint32_t fiftyoneDegreesProfileIterateValuesForProperty(
	const fiftyoneDegreesCollection *values,
	const fiftyoneDegreesProfile *profile,
	const fiftyoneDegreesProperty *property,
	void *state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException *exception);

/**
 * Iterate over all values contained in the profile which relate to the
 * specified property and profile, calling the callback method for each.
 * @param values collection containing all values
 * @param index array of property and profile first value indexes
 * @param profileIndex the index of the profile
 * @param availablePropertyIndex the index of the available property
 * @param property which the values must relate to
 * @param state pointer containing data needed for the callback method
 * @param callback method to be called for each value
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number of matching values which have been iterated
 */
EXTERNAL uint32_t fiftyoneDegreesProfileIterateValuesForPropertyWithIndex(
	const fiftyoneDegreesCollection* values,
	fiftyoneDegreesIndicesPropertyProfile* index,
	uint32_t availablePropertyIndex,
	const fiftyoneDegreesProfile* profile,
	const fiftyoneDegreesProperty* property,
	void* state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException* exception);

/**
 * Iterate all profiles which contain the specified value, calling the callback
 * method for each.
 * @param strings collection containing the strings referenced properties and
 * values
 * @param properties collection containing all properties
 * @param propertyTypes collection containing types for all properties
 * @param values collection containing all values
 * @param profiles collection containing the profiles referenced by the profile
 * offsets
 * @param profileOffsets collection containing all profile offsets (with IDs)
 * @param propertyName name of the property the value relates to
 * @param valueName name of the value to iterate the profiles for
 * @param state pointer to data needed by the callback method
 * @param callback method to be called for each matching profile
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number matching profiles which have been iterated
 */
EXTERNAL uint32_t fiftyoneDegreesProfileIterateProfilesForPropertyWithTypeAndValue(
	fiftyoneDegreesCollection *strings,
	fiftyoneDegreesCollection *properties,
	fiftyoneDegreesCollection *propertyTypes,
	fiftyoneDegreesCollection *values,
	fiftyoneDegreesCollection *profiles,
	fiftyoneDegreesCollection *profileOffsets,
	const char *propertyName,
	const char* valueName,
	void *state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException *exception);

/**
 * Iterate all profiles which contain the specified value, calling the callback
 * method for each.
 * @param strings collection containing the strings referenced properties and
 * values
 * @param properties collection containing all properties
 * @param propertyTypes collection containing types for all properties
 * @param values collection containing all values
 * @param profiles collection containing the profiles referenced by the profile
 * offsets
 * @param profileOffsets collection containing all profile offsets (any form)
 * @param offsetValueExtractor converts `profileOffsets` value to "pure" offset
 * @param propertyName name of the property the value relates to
 * @param valueName name of the value to iterate the profiles for
 * @param state pointer to data needed by the callback method
 * @param callback method to be called for each matching profile
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number matching profiles which have been iterated
 */
EXTERNAL uint32_t fiftyoneDegreesProfileIterateProfilesForPropertyWithTypeAndValueAndOffsetExtractor(
	fiftyoneDegreesCollection *strings,
	fiftyoneDegreesCollection *properties,
	fiftyoneDegreesCollection *propertyTypes,
	fiftyoneDegreesCollection *values,
	fiftyoneDegreesCollection *profiles,
	const fiftyoneDegreesCollection *profileOffsets,
	fiftyoneDegreesProfileOffsetValueExtractor offsetValueExtractor,
	const char *propertyName,
	const char* valueName,
	void *state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException *exception);

/**
 * Iterate all profiles which contain the specified value, calling the callback
 * method for each.
 * @param strings collection containing the strings referenced properties and
 * values
 * @param properties collection containing all properties
 * @param values collection containing all values
 * @param profiles collection containing the profiles referenced by the profile
 * offsets
 * @param profileOffsets collection containing all profile offsets
 * @param propertyName name of the property the value relates to
 * @param valueName name of the value to iterate the profiles for
 * @param state pointer to data needed by the callback method
 * @param callback method to be called for each matching profile
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the number matching profiles which have been iterated
 */
EXTERNAL uint32_t fiftyoneDegreesProfileIterateProfilesForPropertyAndValue(
	fiftyoneDegreesCollection *strings,
	fiftyoneDegreesCollection *properties,
	fiftyoneDegreesCollection *values,
	fiftyoneDegreesCollection *profiles,
	fiftyoneDegreesCollection *profileOffsets,
	const char *propertyName,
	const char* valueName,
	void *state,
	fiftyoneDegreesProfileIterateMethod callback,
	fiftyoneDegreesException *exception);

/**
 * Gets the offset in the profiles collection for the profile with the
 * profileId or NULL if there is no corresponding profile.
 * @param profileOffsets collection containing the profile offsets (with ID)
 * @param profileId the unique id of the profile to fetch
 * @param profileOffset pointer to the integer to set the offset in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the profile offset or NULL
 */
EXTERNAL uint32_t* fiftyoneDegreesProfileGetOffsetForProfileId(
	fiftyoneDegreesCollection *profileOffsets,
	uint32_t profileId,
	uint32_t *profileOffset,
	fiftyoneDegreesException *exception);

/**
 * Gets the profile from the profiles collection
 * with the profileId or NULL if there is no corresponding profile.
 * @param profileOffsets collection containing the profile offsets (without ID)
 * @param profiles collection containing the profiles referenced by the profile
 * offsets
 * @param profileId the unique id of the profile to fetch
 * @param outProfileItem pointer to the item to store profile reference in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the profile or NULL
 */
EXTERNAL fiftyoneDegreesProfile * fiftyoneDegreesProfileGetByProfileIdIndirect(
	fiftyoneDegreesCollection *profileOffsets,
	fiftyoneDegreesCollection *profiles,
	uint32_t profileId,
	fiftyoneDegreesCollectionItem *outProfileItem,
	fiftyoneDegreesException *exception);

/**
 * Calls the callback for every value index available for the profile.
 * @param profile to return value indexes for
 * @param available required properties
 * @param values collection containing all values
 * @param state pointer to data needed by the callback method
 * @param callback method to be called for each value index
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return number of times the callback was called.
 */
EXTERNAL uint32_t fiftyoneDegreesProfileIterateValueIndexes(
	fiftyoneDegreesProfile* profile,
	fiftyoneDegreesPropertiesAvailable* available,
	fiftyoneDegreesCollection* values,
	void* state,
	fiftyoneDegreesProfileIterateValueIndexesMethod callback,
	fiftyoneDegreesException* exception);

/**
 * @}
 */

#endif
