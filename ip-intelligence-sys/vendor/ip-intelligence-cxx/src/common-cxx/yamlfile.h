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

#ifndef FIFTYONE_DEGREES_YAMLFILE_H_INCLUDED
#define FIFTYONE_DEGREES_YAMLFILE_H_INCLUDED

#include <stdio.h>
#include <ctype.h>
#include "status.h"
#include "pair.h"
#include "common.h"

/**
 * APIs for iterating through yaml documents.
 * The yaml document is expected be in a simple format
 *
 * i.e.
 * ---
 * key1: value1
 * key2: value2
 * ---
 * keyn: valuen
 * ...
 * 
 * Syntax:
 * - doc-start ::= (---)
 * - docs-end ::= (...)
 * - key ::= [a-zA-Z0-9.]+
 * - value ::= [a-zA-Z0-9.]+
 * - separtor ::= (:)
 * - pair ::= key separator white-space value
 * - documents ::= document* docs-end
 * - document = doc-start linefeed pair*
 */

/**
 * Iterate through documents up to a given limit in a yaml file.
 * Call the callback function for each document.
 * The document should be structured as documented at the top
 * @param fileName name of the file to be processed
 * @param buffer buffer used to hold data read from the file during the process
 * @param length lengh of th buffer
 * @param keyValuePairs an array of key value pairs used to hold the document
 * key value pairs during the process. Each key and value length should be
 * big enough to hold the biggest  key or value. Else, insufficient memory error
 * will be thrown.
 * @param collectionSize size of the keyValuePairs array
 * @param limit number of document to process
 * @param state the state maintained externally. To be passed to the callback
 * function.
 * @param callback function to call back when a document is read.
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesYamlFileIterateWithLimit(
	const char* fileName,
	char* buffer,
	size_t length,
	fiftyoneDegreesKeyValuePair* keyValuePairs,
	uint16_t collectionSize,
	int limit,
	void* state,
	void(*callback)(fiftyoneDegreesKeyValuePair*, uint16_t, void*));

/**
 * Iterate through documents in a yaml file.
 * Call the callback function for each document.
 * The document should be structured as documented at the top
 * @param fileName name of the file to be processed
 * @param buffer buffer used to hold data read from the file during the process
 * @param length lengh of th buffer
 * @param keyValuePairs an array of key value pairs used to hold the document
 * key value pairs during the process. Each key and value length should be
 * big enough to hold the biggest key or value. Else, insufficient memory error
 * will be thrown.
 * @param collectionSize size of the keyValuePairs array
 * @param state the state maintained externally. To be passed to the callback
 * function.
 * @param callback function to call back when a document is read.
 */
EXTERNAL fiftyoneDegreesStatusCode fiftyoneDegreesYamlFileIterate(
	const char* fileName,
	char* buffer,
	size_t length,
	fiftyoneDegreesKeyValuePair* keyValuePairs,
	uint16_t collectionSize,
	void* state,
	void(*callback)(fiftyoneDegreesKeyValuePair*, uint16_t, void*));

#endif

