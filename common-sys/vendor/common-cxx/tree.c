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

#include "tree.h"

/**
 * RED BLACK BINARY TREE METHODS
 */

/**
 * Macros used in the red black tree within the cache.
 */

#define COLOUR_RED 1 // Indicates the node is black
#define COLOUR_BLACK 0 // Indicates the node is red

/* Gets the root of the tree. The TREE_FIRST macro gets the first node. */
#define TREE_ROOT(n) (&n->root->root)

/* Gets the empty node used to indicate no further data. */
#define TREE_EMPTY(n) &n->root->empty

/* Gets the first node under the root. */
#define TREE_FIRST(n) TREE_ROOT(n)->left

typedef fiftyoneDegreesTreeNode Node;

typedef fiftyoneDegreesTreeIterateMethod IterateMethod;

/**
 * Rotates the red black tree node to the left.
 * @param node pointer to the node being rotated.
 */
static void rotateLeft(Node *node) {
	Node *child = node->right;
	node->right = child->left;

	if (child->left != TREE_EMPTY(node)) {
		child->left->parent = node;
	}
	child->parent = node->parent;

	if (node == node->parent->left) {
		node->parent->left = child;
	}
	else {
		node->parent->right = child;
	}

	child->left = node;
	node->parent = child;
}

/**
 * Rotates the red black tree node to the right.
 * @param node pointer to the node being rotated.
 */
static void rotateRight(Node *node) {
	Node *child = node->left;
	node->left = child->right;

	if (child->right != TREE_EMPTY(node)) {
		child->right->parent = node;
	}
	child->parent = node->parent;

	if (node == node->parent->left) {
		node->parent->left = child;
	}
	else {
		node->parent->right = child;
	}

	child->right = node;
	node->parent = child;
}

/**
 * Maintains the properties of the binary tree following an insert.
 * @param node pointer to the node being repaired after insert.
 */
static void insertRepair(Node *node) {
	Node *uncle;

	while (node->parent->colour == COLOUR_RED) {
		if (node->parent == node->parent->parent->left) {
			uncle = node->parent->parent->right;
			if (uncle->colour == COLOUR_RED) {
				node->parent->colour = COLOUR_BLACK;
				uncle->colour = COLOUR_BLACK;
				node->parent->parent->colour = COLOUR_RED;
				node = node->parent->parent;
			}
			else {
				if (node == node->parent->right) {
					node = node->parent;
					rotateLeft(node);
				}
				node->parent->colour = COLOUR_BLACK;
				node->parent->parent->colour = COLOUR_RED;
				rotateRight(node->parent->parent);
			}
		}
		else {
			uncle = node->parent->parent->left;
			if (uncle->colour == COLOUR_RED) {
				node->parent->colour = COLOUR_BLACK;
				uncle->colour = COLOUR_BLACK;
				node->parent->parent->colour = COLOUR_RED;
				node = node->parent->parent;
			}
			else {
				if (node == node->parent->left) {
					node = node->parent;
					rotateRight(node);
				}
				node->parent->colour = COLOUR_BLACK;
				node->parent->parent->colour = COLOUR_RED;
				rotateLeft(node->parent->parent);
			}
		}
	}
}

/**
 * Finds the successor for the node provided.
 * @param node pointer to the node whose successor is required.
 * @return the successor for the node which may be empty.
 */
static Node* successor(Node *node) {
	Node *successor = node->right;
	if (successor != TREE_EMPTY(node)) {
		while (successor->left != TREE_EMPTY(node)) {
			successor = successor->left;
		}
	}
	else {
		for (successor = node->parent;
			node == successor->right;
			successor = successor->parent) {
			node = successor;
		}
		if (successor == TREE_ROOT(node)) {
			successor = TREE_EMPTY(node);
		}
	}
	return successor;
}

/**
 * Following a deletion repair the section of the tree impacted.
 * @param node pointer to the node below the one deleted.
 */
static void deleteRepair(Node *node) {
	Node *sibling;

	while (node->colour == COLOUR_BLACK && node != TREE_FIRST(node)) {
		if (node == node->parent->left) {
			sibling = node->parent->right;
			if (sibling->colour == COLOUR_RED) {
				sibling->colour = COLOUR_BLACK;
				node->parent->colour = COLOUR_RED;
				rotateLeft(node->parent);
				sibling = node->parent->right;
			}
			if (sibling->right->colour == COLOUR_BLACK &&
				sibling->left->colour == COLOUR_BLACK) {
				sibling->colour = COLOUR_RED;
				node = node->parent;
			}
			else {
				if (sibling->right->colour == COLOUR_BLACK) {
					sibling->left->colour = COLOUR_BLACK;
					sibling->colour = COLOUR_RED;
					rotateRight(sibling);
					sibling = node->parent->right;
				}
				sibling->colour = node->parent->colour;
				node->parent->colour = COLOUR_BLACK;
				sibling->right->colour = COLOUR_BLACK;
				rotateLeft(node->parent);
				node = TREE_FIRST(node);
			}
		}
		else {
			sibling = node->parent->left;
			if (sibling->colour == COLOUR_RED) {
				sibling->colour = COLOUR_BLACK;
				node->parent->colour = COLOUR_RED;
				rotateRight(node->parent);
				sibling = node->parent->left;
			}
			if (sibling->right->colour == COLOUR_BLACK &&
				sibling->left->colour == COLOUR_BLACK) {
				sibling->colour = COLOUR_RED;
				node = node->parent;
			}
			else {
				if (sibling->left->colour == COLOUR_BLACK) {
					sibling->right->colour = COLOUR_BLACK;
					sibling->colour = COLOUR_RED;
					rotateLeft(sibling);
					sibling = node->parent->left;
				}
				sibling->colour = node->parent->colour;
				node->parent->colour = COLOUR_BLACK;
				sibling->left->colour = COLOUR_BLACK;
				rotateRight(node->parent);
				node = TREE_FIRST(node);
			}
		}
	}
	node->colour = COLOUR_BLACK;
}

#ifdef _MSC_VER
#pragma warning (disable: 4100) 
#endif
static void increaseCount(void *state, Node *node) {
	(*(uint32_t*)state)++;
}
#ifdef _MSC_VER
#pragma warning (default: 4100) 
#endif

static void iterate(Node *node, void *state, IterateMethod callback) {
	if (node != TREE_EMPTY(node)) {
		if (node->left != TREE_EMPTY(node)) {
			assert(node->key > node->left->key);
			iterate(node->left, state, callback);
		}
		if (node->right != TREE_EMPTY(node)) {
			assert(node->key < node->right->key);
			iterate(node->right, state, callback);
		}
		callback(state, node);
	}
}

fiftyoneDegreesTreeNode* fiftyoneDegreesTreeFind(
	fiftyoneDegreesTreeRoot *root, 
	int64_t key) {
	Node *node = root->root.left;

	while (node != TREE_EMPTY(node)) {
		if (key == node->key) {
			return node;
		}
		node = key < node->key ? node->left : node->right;
	}

	return NULL;
}

void fiftyoneDegreesTreeInsert(fiftyoneDegreesTreeNode *node) {
	Node *current = TREE_FIRST(node);
	Node *parent = TREE_ROOT(node);

	// Work out the correct point to insert the node.
	while (current != TREE_EMPTY(node)) {
		parent = current;
		assert(node->key != current->key);
		current = node->key < current->key
			? current->left
			: current->right;
	}
	
	// Set up the node being inserted in the tree.
	current = (Node*)node;
	current->left = TREE_EMPTY(node);
	current->right = TREE_EMPTY(node);
	current->parent = parent;
	if (parent == TREE_ROOT(node) ||
		current->key < parent->key) {
		parent->left = current;
	}
	else {
		parent->right = current;
	}
	current->colour = COLOUR_RED;
	insertRepair(current);
	
	TREE_FIRST(current)->colour = COLOUR_BLACK;
}

void fiftyoneDegreesTreeDelete(fiftyoneDegreesTreeNode *node) {
	Node *x, *y;

	if (node->left == TREE_EMPTY(node) ||
		node->right == TREE_EMPTY(node)) {
		y = node;
	}
	else {
		y = successor(node);
	}
	x = y->left == TREE_EMPTY(node) ? y->right : y->left;

	x->parent = y->parent;
	if (x->parent == TREE_ROOT(node)) {
		TREE_FIRST(node) = x;
	}
	else {
		if (y == y->parent->left) {
			y->parent->left = x;
		}
		else {
			y->parent->right = x;
		}
	}

	if (y->colour == COLOUR_BLACK) {
		deleteRepair(x);
	}
	if (y != node) {
		y->left = node->left;
		y->right = node->right;
		y->parent = node->parent;
		y->colour = node->colour;
		node->left->parent = y;
		node->right->parent = y;
		if (node == node->parent->left) {
			node->parent->left = y;
		}
		else {
			node->parent->right = y;
		}
	}
}

void fiftyoneDegreesTreeNodeRemove(fiftyoneDegreesTreeNode *node) {
	node->left = TREE_EMPTY(node);
	node->right = TREE_EMPTY(node);
	node->parent = TREE_EMPTY(node);
	node->colour = COLOUR_BLACK;
}

void fiftyoneDegreesTreeRootInit(fiftyoneDegreesTreeRoot *root) {
	root->empty.colour = COLOUR_BLACK;
	root->empty.root = root;
	root->empty.left = &root->empty;
	root->empty.right = &root->empty;
	root->empty.parent = NULL;
	root->empty.key = 0;
	root->root.colour = COLOUR_BLACK;
	root->root.root = root;
	root->root.left = &root->empty;
	root->root.right = &root->empty;
	root->root.parent = NULL;
	root->root.key = 0;
}

void fiftyoneDegreesTreeNodeInit(
	fiftyoneDegreesTreeNode *node, 
	fiftyoneDegreesTreeRoot *root) {
	node->root = root;
	node->key = 0;
	fiftyoneDegreesTreeNodeRemove(node);
}

void fiftyoneDegreesTreeIterateNodes(
	fiftyoneDegreesTreeRoot *root,
	void *state,
	fiftyoneDegreesTreeIterateMethod callback) {
	iterate(root->root.left, state, callback);
}

uint32_t fiftyoneDegreesTreeCount(fiftyoneDegreesTreeRoot *root) {
	uint32_t count = 0;
	iterate(root->root.left, &count, increaseCount);
	return count;
}

