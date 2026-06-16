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

#ifndef FIFTYONE_DEGREES_OVERRIDES_INCLUDED
#define FIFTYONE_DEGREES_OVERRIDES_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesOverrides Overrides
 *
 * Used to override properties values or an entire profile.
 *
 * ## Introduction
 *
 * Overrides are used to override property values, or an entire profile in a
 * set of results. For example, if the screen size is not known from the HTTP
 * headers alone, it can be fetched using JavaScript and overridden.
 *
 * ## Creation
 *
 * An array of overridable properties is created using the
 * #fiftyoneDegreesOverridePropertiesCreate method. This returns all the
 * properties which are capable of being overridden.
 *
 * An array of override value ready to be populated is created using the
 * #fiftyoneDegreesOverrideValuesCreate method. This is then ready to be added
 * to and used to override the values in a results structure.
 *
 * ## Extraction
 *
 * Override values are extracted from an evidence structure using the
 * #fiftyoneDegreesOverridesExtractFromEvidence method. This looks through the
 * items of evidence for any items which are overrides, then parses and adds
 * them to the override values ready to be applied.
 *
 * ## Add
 *
 * Override values can also be added using the #fiftyoneDegreesOverridesAdd or
 * #fiftyoneDegreesOverrideValuesAdd methods which add a single or multiple 
 * values respectively to the override values.
 *
 * ## Free
 *
 * Property and value overrides are freed using the
 * #fiftyoneDegreesOverridePropertiesFree and
 * #fiftyoneDegreesOverrideValuesFree methods.
 *
 * @{
 */

#include <stdint.h>
#include <stdbool.h>
#include "data.h"
#include "string.h"
#include "collection.h"
#include "properties.h"
#include "evidence.h"
#include "array.h"
#include "common.h"

/**
 * Index and pointer to a property which can be overridden.
 */
typedef struct fiftyone_degrees_override_property_t {
	uint32_t requiredPropertyIndex; /**< Index in the available properties
									structure */
	fiftyoneDegreesPropertyAvailable *available; /**< Pointer to the property */
} fiftyoneDegreesOverrideProperty;

/**
 * String value which should override the value for the property indicated by
 * the required property index.
 */
typedef struct fiftyone_degrees_override_value_t {
	uint32_t requiredPropertyIndex; /**< Index in the available properties
									structure */
	fiftyoneDegreesData string; /**< Overridden value */
} fiftyoneDegreesOverrideValue;

FIFTYONE_DEGREES_ARRAY_TYPE(
	fiftyoneDegreesOverrideProperty,
	bool prefix; /**< Flag which when true requires the `51D_` prefix to be
				 checked for in evidence. */
);

/**
 * An array of properties and values to use when getting override values.
 */
FIFTYONE_DEGREES_ARRAY_TYPE(fiftyoneDegreesOverrideValue,);

/**
 * Array of overridable properties. These are properties in a data set which
 * a capable of being overridden.
 */
typedef fiftyoneDegreesOverridePropertyArray fiftyoneDegreesOverridable;

/**
 * Called when a profile Id has been found and should override the one
 * found from another source such as the `User-Agent`.
 * @param state pointer to be used by the method
 * @param profileId if to override
 */
typedef void(*fiftyoneDegreesOverrideProfileIdMethod)(
	void *state, 
	uint32_t profileId);

/**
 * Determines if the required property index is eligible for overriding 
 * considering the configuration of the data set.
 * @param state pointer to be used by the method
 * @param requiredPropertyIndex index of the property in the available
 * properties structure
 * @return true if the property can be overridden, otherwise false.
 */
typedef bool(*fiftyoneDegreesOverridesFilterMethod)(
	void *state,
	uint32_t requiredPropertyIndex);

/**
 * Creates a fresh array of override values with the given capacity.
 * @param capacity the number of values the array can contain
 * @return a new array of override values
 */
EXTERNAL fiftyoneDegreesOverrideValueArray* fiftyoneDegreesOverrideValuesCreate(
	uint32_t capacity);

/**
 * Returns a list of the evidence keys that are available to support 
 * overriding property values.
 * @param available pointer to the available properties structure
 * @param prefix true if the `51D_` prefix should be checked for in evidence
 * @param state pointer to be used by the method
 * @param filter pointer to a filter method which determines whether or not a
 * property is eligible to be overridden
 * @return a new override properties array
 */
fiftyoneDegreesOverridePropertyArray* 
fiftyoneDegreesOverridePropertiesCreate(
	fiftyoneDegreesPropertiesAvailable *available,
	bool prefix,
	void *state,
	fiftyoneDegreesOverridesFilterMethod filter);

/**
 * Frees the resources used by the override properties.
 * @param properties pointer to the properties to free
 */
void fiftyoneDegreesOverridePropertiesFree(
	fiftyoneDegreesOverridePropertyArray *properties);

/**
 * Extracts override values from evidence.
 * @param properties which can be overridden
 * @param values array to populate with the override values
 * @param evidence to extract any overrides from
 * @return the number of override values which have been extracted
 */
 uint32_t fiftyoneDegreesOverridesExtractFromEvidence(
	fiftyoneDegreesOverridePropertyArray *properties,
	fiftyoneDegreesOverrideValueArray *values,
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence);

/**
 * Gets whether or not the override values contain an override relating to the
 * property identified by the required property index. Note that the required
 * property index MUST be valid when calling this method, so should be checked
 * by the caller.
 * @param values pointer to the values to search in
 * @param requiredPropertyIndex index in the required properties of the 
 * property to check for values of
 * @return true if there are override values for the requested property
 */
 bool fiftyoneDegreesOverrideHasValueForRequiredPropertyIndex(
	 fiftyoneDegreesOverrideValueArray *values,
	 uint32_t requiredPropertyIndex);

/**
 * Adds values from the overrides data structure into the results.
 * @param values array to add the new values to
 * @param requiredPropertyIndex of the property which the values relate to
 * @param list to add the values to
 * @return the number of values which have been added to the list
 */
uint32_t fiftyoneDegreesOverrideValuesAdd(
	fiftyoneDegreesOverrideValueArray *values,
	uint32_t requiredPropertyIndex,
	fiftyoneDegreesList *list);

/**
 * Add an value override to the override values array.
 * @param values the override values array to add the value to
 * @param requiredPropertyIndex the index in the dataset's required properties
 * of the property to override the value of
 * @param value the value string override
 * @return true if the value was added successfully
 */
EXTERNAL bool fiftyoneDegreesOverridesAdd(
	fiftyoneDegreesOverrideValueArray *values,
	int requiredPropertyIndex,
	const char *value);
	
/**
 * Returns the first value for the required property index or NULL of no value
 * exists for the property index.
 * @param values to get the value from
 * @param requiredPropertyIndex index of the property to get the first value of
 * @param item to store the result in
 * @return pointer to the value or NULL if none were found
 */
fiftyoneDegreesString* fiftyoneDegreesOverrideValuesGetFirst(
	fiftyoneDegreesOverrideValueArray *values,
	uint32_t requiredPropertyIndex,
	fiftyoneDegreesCollectionItem *item);

/**
 * Gets the required property index of a property that **MIGHT** provide logic
 * such as JavaScript or Regular Expressions that could override the value
 * of the required property index provided.
 * @param available list of require properties
 * @param requiredPropertyIndex of the property to test for being overridden
 * @return the positive index of the overriding required property, or -1 it 
 * can't be overridden.
 */
int fiftyoneDegreesOverridesGetOverridingRequiredPropertyIndex(
	fiftyoneDegreesPropertiesAvailable *available,
	uint32_t requiredPropertyIndex);

/**
 * Frees the memory used for the override values.
 * @param values to be freed
 */
EXTERNAL void fiftyoneDegreesOverrideValuesFree(
	fiftyoneDegreesOverrideValueArray *values);

/**
 * Reset override array. All existing item memory will not be freed by reset
 * with 0s. Remaining values will be reset to default except the allocateds size..
 * @param values to be reset
 */
EXTERNAL void fiftyoneDegreesOverrideValuesReset(
	fiftyoneDegreesOverrideValueArray *values);

/**
 * Extracts the profile overrides from the evidence and overrides them using
 * the override method supplied.
 * @param evidence to extract the profile ids from
 * @param state pointer to pass to the override method
 * @param callback method called to override a profile id
 */
EXTERNAL void fiftyoneDegreesOverrideProfileIds(
	fiftyoneDegreesEvidenceKeyValuePairArray *evidence, 
	void *state, 
	fiftyoneDegreesOverrideProfileIdMethod callback);

/**
 * @}
 */

#endif
