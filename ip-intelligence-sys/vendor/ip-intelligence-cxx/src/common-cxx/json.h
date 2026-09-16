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

#ifndef FIFTYONE_DEGREES_JSON_H_INCLUDED
#define FIFTYONE_DEGREES_JSON_H_INCLUDED

 /**
  * @ingroup FiftyOneDegreesCommon
  * @defgroup FiftyOneDegreesJson JSON
  *
  * JSON methods
  *
  * ## Introduction
  * 
  * Contains common methods to create JSON documents, add properties, add 
  * either single or list values to properties. String values are escaped to
  * comply with the JSON specification.
  * 
  * ## Data Structures
  * 
  * A single data structure is used with members for a) the output buffer, and 
  * b) the reference data. 
  * 
  * The output buffer is represented as a pointer and a length. An additional
  * member is used to record the number of characters that would be needed to
  * complete the creation of a valid JSON response. This can be used by the 
  * caller to increase the buffer size if not big enough and call the related
  * methods a subsequent time. 
  * 
  * Reference data for the property being added, the values being added, and
  * a collection of strings is also provided.
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
#include "property.h"
#include "string.h"
#include "list.h"
#include "data.h"
#include "collection.h"
#include "common.h"
#include "exceptions.h"

/**
 * Structure used to populated a JSON string for all required properties and 
 * values. The implementation will always check to determine if sufficient 
 * characters remain in the buffer before adding characters. The charsAdded
 * field is always updated to reflect the number of characters that would be
 * needed in the buffer if all the values were to be written. This is needed
 * to determine when the buffer provided needs to be larger.
 */
typedef struct fiftyone_degrees_json {
	fiftyoneDegreesStringBuilder builder; /**< Output buffer */
	fiftyoneDegreesCollection* strings; /**< Collection of strings */
	fiftyoneDegreesProperty* property; /**< The property being added */
	fiftyoneDegreesList* values; /**< The values for the property */
	fiftyoneDegreesException* exception; /**< Exception */
	fiftyoneDegreesPropertyValueType storedPropertyType; /**< Stored type of the values for the property */
} fiftyoneDegreesJson;

/**
 * Writes the start of the JSON document characters to the buffer in json.
 * @param json data structure
 */
EXTERNAL void fiftyoneDegreesJsonDocumentStart(fiftyoneDegreesJson* json);

/**
 * Writes the end of the JSON document characters to the buffer in json.
 * @param json data structure
 */
EXTERNAL void fiftyoneDegreesJsonDocumentEnd(fiftyoneDegreesJson* json);

/**
 * Writes the start of the property in json->property to the buffer in json.
 * @param json data structure
 */
EXTERNAL void fiftyoneDegreesJsonPropertyStart(fiftyoneDegreesJson* json);

/**
 * Writes the end of the property in json->property to the buffer in json.
 * @param json data structure
 */
EXTERNAL void fiftyoneDegreesJsonPropertyEnd(fiftyoneDegreesJson* json);

/**
 * Writes the values in the json->values list to the buffer in json.
 * @param json data structure
 */
EXTERNAL void fiftyoneDegreesJsonPropertyValues(fiftyoneDegreesJson* json);

/**
 * Writes the a property separator to the buffer in json.
 * @param json data structure
 */
EXTERNAL void fiftyoneDegreesJsonPropertySeparator(fiftyoneDegreesJson* json);

/**
 * @}
 */

#endif
