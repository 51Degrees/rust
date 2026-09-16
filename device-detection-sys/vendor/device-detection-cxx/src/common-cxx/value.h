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

#ifndef FIFTYONE_DEGREES_PROPERTY_VALUE_H_INCLUDED
#define FIFTYONE_DEGREES_PROPERTY_VALUE_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesValue Values
 *
 * Value of a data set relating to a property.
 *
 * ## Introduction
 *
 * A Value is stored in a values collection and contains the meta data for a
 * specific value of a property in a data set.
 *
 * ## Get
 *
 * A value can be fetched from a values collection in one of two ways:
 *
 * **By Index** : the #fiftyoneDegreesValueGet method return the value at a
 * specified index. This provides a way to access a value at a known index, or
 * iterate over all values.
 *
 * **By Name** : if the index of a value is not known, then the value can be
 * fetched using the #fiftyoneDegreesValueGetByName method to find the value in
 * a values collection. This requires that the property the value relates to is
 * also known, as values are only unique within the values which relate to a
 * single property. For example "True" could be a value of many properties.
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
#include "storedBinaryValue.h"
#include "property.h"
#include "profile.h"
#include "common.h"

/**
 * Macro to check if a Value's urlOffsetOrWeight field carries a masked weight.
 * A value is weighted when (urlOffsetOrWeight & 0xFFFF0000) == 0xFF000000.
 * This is distinct from -1 (0xFFFFFFFF, "no URL" sentinel) and from
 * valid non-negative URL offsets.
 */
#define FIFTYONE_DEGREES_VALUE_IS_MASKED(v) \
	((((uint32_t)((v)->urlOffsetOrWeight)) & 0xFFFF0000u) == 0xFF000000u)

/** Value structure containing meta data relating to the value. */
#pragma pack(push, 2)
typedef struct fiftyoneDegrees_value_t {
	const int16_t propertyIndex; /**< Index of the property the value relates to */
	const int32_t nameOffset; /**< The offset in the strings structure to the 
	                              value name */
#ifndef FIFTYONE_DEGREES_REDUCED_FILE
	// Descriptions are not included in reduced size data files.
	const int32_t descriptionOffset; /**< The offset in the strings structure to
	                                     the value description */
#endif
	const int32_t urlOffsetOrWeight; /**< The offset in the strings structure to
	                                     the value URL, or a masked weight if
	                                     upper 2 bytes == 0xFF00 (i.e. value
	                                     has a form of `0xFF00****` where
	                                     `****` are weight value bits).
	                                     See fiftyoneDegreesValueIsWeighted(). */
} fiftyoneDegreesValue;
#pragma pack(pop)

/**
 * Returns the contents of the value using the item provided. The
 * collection item must be released when the caller is finished with the
 * string.
 * @param strings collection of strings retrieved by offsets.
 * @param value structure for the name required.
 * @param storedValueType format of byte array representation.
 * @param item used to store the resulting string in.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to a contents in the collection item data structure.
 */
EXTERNAL const fiftyoneDegreesStoredBinaryValue* fiftyoneDegreesValueGetContent(
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesValue *value,
	fiftyoneDegreesPropertyValueType storedValueType,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Returns the string name of the value using the item provided. The
 * collection item must be released when the caller is finished with the
 * string.
 * @param strings collection of strings retrieved by offsets.
 * @param value structure for the name required.
 * @param item used to store the resulting string in.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to a string in the collection item data structure.
 */
EXTERNAL const fiftyoneDegreesString* fiftyoneDegreesValueGetName(
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesValue *value,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Returns the string description of the value using the item provided. The
 * collection item must be released when the caller is finished with the
 * string.
 * @param strings collection of strings retrieved by offsets.
 * @param value structure for the description required.
 * @param item used to store the resulting string in.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to a string in the collection item data structure.
 */
EXTERNAL const fiftyoneDegreesString* fiftyoneDegreesValueGetDescription(
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesValue *value,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Returns the string URL of the value using the item provided. The
 * collection item must be released when the caller is finished with the
 * string.
 * @param strings collection of strings retrieved by offsets.
 * @param value structure for the URL required.
 * @param item used to store the resulting string in.
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return a pointer to a string in the collection item data structure.
 */
EXTERNAL const fiftyoneDegreesString* fiftyoneDegreesValueGetUrl(
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesValue *value,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Gets the value for the requested index from the collection provided.
 * @param values collection to get the value from
 * @param valueIndex index of the value to get
 * @param item to store the value in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h.
 * @return pointer to the value or NULL
 */
EXTERNAL const fiftyoneDegreesValue* fiftyoneDegreesValueGet(
	const fiftyoneDegreesCollection *values,
	uint32_t valueIndex,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Get the value for the requested name from the collection provided.
 * @param values collection to get the value from
 * @param strings collection containing the value names
 * @param property that the value relates to
 * @param storedValueType format of byte array representation.
 * @param valueName name of the value to get
 * @param item to store the value in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the value or NULL if it does not exist
 */
EXTERNAL const fiftyoneDegreesValue* fiftyoneDegreesValueGetByNameAndType(
	const fiftyoneDegreesCollection *values,
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesProperty *property,
	fiftyoneDegreesPropertyValueType storedValueType,
	const char *valueName,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Get the value for the requested name from the collection provided.
 * @param values collection to get the value from
 * @param strings collection containing the value names
 * @param property that the value relates to
 * @param valueName name of the value to get
 * @param item to store the value in
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return pointer to the value or NULL if it does not exist
 */
EXTERNAL const fiftyoneDegreesValue* fiftyoneDegreesValueGetByName(
	const fiftyoneDegreesCollection *values,
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesProperty *property,
	const char *valueName,
	fiftyoneDegreesCollectionItem *item,
	fiftyoneDegreesException *exception);

/**
 * Get index of the value for the requested name from the collection provided.
 * @param values collection to get the value from
 * @param strings collection containing the value names
 * @param property that the value relates to
 * @param storedValueType format of byte array representation
 * @param valueName name of the value to get
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the 0 based index of the item if found, otherwise -1
 */
EXTERNAL long fiftyoneDegreesValueGetIndexByNameAndType(
	const fiftyoneDegreesCollection *values,
	const fiftyoneDegreesCollection *strings,
	const fiftyoneDegreesProperty *property,
	fiftyoneDegreesPropertyValueType storedValueType,
	const char *valueName,
	fiftyoneDegreesException *exception);

/**
 * Get index of the value for the requested name from the collection provided.
 * @param values collection to get the value from
 * @param strings collection containing the value names
 * @param property that the value relates to
 * @param valueName name of the value to get
 * @param exception pointer to an exception data structure to be used if an
 * exception occurs. See exceptions.h
 * @return the 0 based index of the item if found, otherwise -1
 */
EXTERNAL long fiftyoneDegreesValueGetIndexByName(
	fiftyoneDegreesCollection *values,
	fiftyoneDegreesCollection *strings,
	fiftyoneDegreesProperty *property,
	const char *valueName,
	fiftyoneDegreesException *exception);

/**
 * Returns true if the value carries a masked weight.
 * A value is weighted when (urlOffsetOrWeight & 0xFFFF0000) == 0xFF000000.
 * This is distinct from -1 (0xFFFFFFFF, "no URL" sentinel) and from
 * valid non-negative URL offsets.
 * @param value the value to check
 * @return true if the value carries a masked weight, false otherwise
 */
EXTERNAL bool fiftyoneDegreesValueIsWeighted(
	const fiftyoneDegreesValue *value);

/**
 * Gets the weight from a Value record as a uint16_t (0–65535),
 * or 0 if the value is not weighted.
 * The weight is stored in the lower 2 bytes of urlOffsetOrWeight
 * when the upper 2 bytes match the 0xFF00 mask.
 * To convert to a proportion: (double)weight / UINT16_MAX
 * @param value the value to get the weight from
 * @return the weight (0–65535 range), or 0 if not weighted
 */
EXTERNAL uint16_t fiftyoneDegreesValueGetWeight(
	const fiftyoneDegreesValue *value);

/**
 * @}
 */

#endif
