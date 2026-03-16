/**
    ******************************************************************************
    * @file           : board.h
    * @brief          : Board-level configuration header.
    *                   This file contains common board definitions and error handling.
    ******************************************************************************
    */
#ifndef __BOARD_H
#define __BOARD_H

#ifdef __cplusplus
extern "C" {
#endif

#include "stm32h7xx_hal.h"

void Error_Handler(void);

#define LED_Pin GPIO_PIN_13
#define LED_GPIO_Port GPIOC

#ifdef __cplusplus
}
#endif

#endif /* __BOARD_H */
