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

#ifndef FIFTYONE_DEGREES_TEXT_FILE_H_INCLUDED
#define FIFTYONE_DEGREES_TEXT_FILE_H_INCLUDED

/**
 * @ingroup FiftyOneDegreesCommon
 * @defgroup FiftyOneDegreesTextFile TextFile
 *
 * Contains helper methods for accessing and using text files. This is of most
 * use in unit tests and examples, where text files containing sample data are
 * used.
 *
 * @{
 */

#include <stdio.h>
#include <ctype.h>
#include "file.h"
#include "common.h"

/**
 * Iterates over all the lines in a text file up to the given limit number of
 * line to read, calling the callback method with each line.
 * @param fileName name of the file to iterate over
 * @param buffer to use for reading lines into. The buffer needs
 * to be big enough to hold the biggest record, including its line ending.
 * @param length of the buffer
 * @param limit number of lines to read. -1 for read all.
 * @param state pointer to pass to the callback method
 * @param callback method to call with each line
 */
EXTERNAL void fiftyoneDegreesTextFileIterateWithLimit(
	const char *fileName, 
	char *buffer, 
	int length, 
	int limit,
	void *state,
	void(*callback)(const char*, void*));

/**
 * Iterates over all the lines in a text file calling the callback method with
 * each line.
 * @param fileName name of the file to iterate over
 * @param buffer to use for reading lines into. The buffer needs
 * to be big enough to hold the biggest record, including its line ending.
 * @param length of the buffer
 * @param state pointer to pass to the callback method
 * @param callback method to call with each line
 */
EXTERNAL void fiftyoneDegreesTextFileIterate(
	const char *fileName, 
	char *buffer, 
	int length, 
	void *state,
	void(*callback)(const char*, void*));

/**
 * @}
 */

#endif